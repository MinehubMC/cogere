use axum::{
    Json,
    extract::{Multipart, Path, State},
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedEntity,
        permissions::{Action, PermissionCheck, ResourceType, check::PermissionChecker},
    },
    database,
    errors::{AppError, Error},
    server::AppState,
    storage::LocalStorage,
};

#[derive(Debug, Deserialize)]
pub struct PluginMetadata {
    pub artifact_id: String,
    pub group_id: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct PluginUploadResponse {
    pub id: Uuid,
    pub artifact_id: String,
    pub group_id: String,
    pub version: String,
}

pub async fn plugin_upload(
    State(state): State<AppState>,
    entity: AuthenticatedEntity,
    Path(group_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<PluginUploadResponse>, AppError> {
    PermissionChecker::new(&state.db, &entity)
        .require(PermissionCheck::on_type(ResourceType::Plugin, Action::Create).in_group(group_id))
        .await?;

    tracing::debug!(
        "Plugin upload started for entity: {:?}",
        entity.identifier()
    );

    let mut plugin_file: Option<Vec<u8>> = None;
    let mut metadata: Option<PluginMetadata> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?
    {
        let name = field
            .name()
            .ok_or_else(|| Error::BadRequest("multipart field has no name".into()))?
            .to_owned();

        tracing::debug!("Processing multipart field: {}", name);

        match name.as_str() {
            "file" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| Error::BadRequest(e.to_string()))?;
                tracing::debug!("Received file with {} bytes", data.len());
                plugin_file = Some(data.to_vec());
            }
            "metadata" => {
                let data = field
                    .text()
                    .await
                    .map_err(|e| Error::BadRequest(e.to_string()))?;
                tracing::debug!("Received metadata: {}", data);
                metadata = Some(
                    serde_json::from_str(&data).map_err(|e| Error::BadRequest(e.to_string()))?,
                );
            }
            other => {
                tracing::debug!("Ignoring unknown field: {}", other);
            }
        }
    }

    let plugin_file = plugin_file.ok_or_else(|| Error::BadRequest("no file provided".into()))?;
    let metadata = metadata.ok_or_else(|| Error::BadRequest("no metadata provided".into()))?;

    if plugin_file.is_empty() {
        return Err(Error::BadRequest("uploaded file is empty".into()).into());
    }

    let id = Uuid::now_v7();
    tracing::debug!("Generated plugin ID: {}", id);

    state.storage.put(id, Bytes::from(plugin_file)).await?;

    database::plugins::create_plugin(
        &state.db,
        id,
        &metadata.artifact_id,
        &metadata.group_id,
        &metadata.version,
    )
    .await?;

    tracing::info!(
        "Plugin uploaded successfully: {} {}:{}",
        metadata.artifact_id,
        metadata.group_id,
        metadata.version
    );

    Ok(Json(PluginUploadResponse {
        id,
        artifact_id: metadata.artifact_id,
        group_id: metadata.group_id,
        version: metadata.version,
    }))
}
