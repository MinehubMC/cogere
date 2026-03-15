use bytes::Bytes;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedEntity,
        permissions::{Action, PermissionCheck, ResourceType, check::PermissionChecker},
    },
    database,
    errors::Error,
    server::AppState,
    storage::LocalStorage,
};

pub struct UploadPluginOptions {
    pub group_id: Uuid,
    pub plugin_group_id: String,
    pub plugin_artifact_id: String,
    pub version: String,
    pub file: Bytes,
}

pub struct UploadPluginOutput {
    pub plugin_id: Uuid,
    pub version_id: Uuid,
}

pub async fn upload_plugin(
    state: &AppState,
    entity: &AuthenticatedEntity,
    input: UploadPluginOptions,
) -> Result<UploadPluginOutput, Error> {
    PermissionChecker::new(&state.db, &entity)
        .require(
            PermissionCheck::new(ResourceType::Plugin, Action::Create).in_group(input.group_id),
        )
        .await?;

    let existing_version = database::plugins::get_plugin_version(
        &state.db,
        input.group_id,
        input.plugin_group_id.clone(),
        input.plugin_artifact_id.clone(),
        input.version.clone(),
    )
    .await?;

    if let Some(plugin_version) = existing_version {
        return Err(Error::Conflict(format!(
            "version already exists with id: {0}",
            plugin_version.id
        )));
    }

    if input.file.is_empty() {
        return Err(Error::BadRequest("uploaded file is empty".into()));
    }
    let sha256 = {
        let mut hasher = Sha256::new();
        hasher.update(&input.file);
        format!("{:x}", hasher.finalize())
    };
    let size_bytes = input.file.len() as u64;

    let existing_blob = database::blobs::find_by_sha256(&state.db, sha256.clone()).await?;
    let is_new_blob = existing_blob.is_none();

    if is_new_blob {
        let group = database::groups::get_group_by_id(&state.db, input.group_id)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!("Group with id='{}' not found", input.group_id))
            })?;

        if group.quota_would_exceed(size_bytes) {
            return Err(Error::BadRequest(format!(
                "upload would exceed group quota ({} bytes available)",
                group.quota_available_bytes().unwrap_or(0)
            )));
        }
    }

    let blob_id = match existing_blob {
        Some(blob) => blob.id,
        None => Uuid::now_v7(),
    };
    let plugin_id = Uuid::now_v7();
    let version_id = Uuid::now_v7();

    if is_new_blob {
        state.storage.put(blob_id, input.file.clone()).await?;
    }

    let db_result = database::plugins::create_local_plugin(
        &state.db,
        database::plugins::CreateLocalPluginOptions {
            plugin_id,
            version_id,
            blob_id,
            group_id: input.group_id,
            plugin_group_id: input.plugin_group_id.clone(),
            plugin_artifact_id: input.plugin_artifact_id.clone(),
            version: input.version.clone(),
            sha256: sha256.clone(),
            size_bytes,
            is_new_blob,
        },
    )
    .await;

    if let Err(e) = db_result {
        if is_new_blob {
            if let Err(storage_err) = state.storage.delete(blob_id).await {
                tracing::error!(
                    blob_id = %blob_id,
                    error = %storage_err,
                    "failed to clean up blob from storage after DB failure"
                );
            }
        }

        return Err(e.into());
    }

    tracing::info!(
        plugin_id = %plugin_id,
        group_id = %input.group_id,
        artifact = format!("{}:{}", input.plugin_group_id, input.plugin_artifact_id),
        "plugin uploaded successfully"
    );

    Ok(UploadPluginOutput {
        plugin_id,
        version_id,
    })
}
