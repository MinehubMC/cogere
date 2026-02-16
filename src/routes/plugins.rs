use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedEntity,
        permissions::{HasPermissions, Permission},
    },
    database,
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
    mut multipart: Multipart,
) -> Result<Json<PluginUploadResponse>, StatusCode> {
    tracing::debug!(
        "Plugin upload started for entity: {:?}",
        entity.identifier()
    );

    if !entity.has_permission(Permission::UploadPlugin) {
        tracing::warn!("Permission denied for entity: {}", entity.identifier());
        return Err(StatusCode::FORBIDDEN);
    }

    let mut plugin_file: Option<Vec<u8>> = None;
    let mut metadata: Option<PluginMetadata> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to get next multipart field: {:?}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().ok_or_else(|| {
            tracing::error!("Multipart field has no name");
            StatusCode::BAD_REQUEST
        })?;

        tracing::debug!("Processing multipart field: {}", name);

        match name {
            "file" => {
                let data = field.bytes().await.map_err(|e| {
                    tracing::error!("Failed to read file bytes: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?;
                tracing::debug!("Received file with {} bytes", data.len());
                plugin_file = Some(data.to_vec());
            }
            "metadata" => {
                let data = field.text().await.map_err(|e| {
                    tracing::error!("Failed to read metadata text: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?;
                tracing::debug!("Received metadata: {}", data);
                metadata = Some(serde_json::from_str(&data).map_err(|e| {
                    tracing::error!("Failed to parse metadata JSON: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?);
            }
            _ => {
                tracing::debug!("Ignoring unknown field: {}", name);
            }
        }
    }

    let plugin_file = plugin_file.ok_or_else(|| {
        tracing::error!("No file provided in upload");
        StatusCode::BAD_REQUEST
    })?;
    let metadata = metadata.ok_or_else(|| {
        tracing::error!("No metadata provided in upload");
        StatusCode::BAD_REQUEST
    })?;

    if plugin_file.is_empty() {
        tracing::error!("Uploaded file is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::now_v7();
    tracing::debug!("Generated plugin ID: {}", id);

    state
        .storage
        .put(id, Bytes::from(plugin_file))
        .await
        .map_err(|e| {
            tracing::error!("Storage error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    database::plugins::create_plugin(
        &state.db,
        id,
        &metadata.artifact_id,
        &metadata.group_id,
        &metadata.version,
    )
    .await
    .map_err(|e| {
        tracing::error!("Database error: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

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
