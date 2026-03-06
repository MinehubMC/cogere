use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::extractor::AuthenticatedEntity, server::AppState};

#[derive(Clone, Deserialize)]
pub struct RequestAssemblyArtifact {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
}

#[derive(Clone, Deserialize)]
pub struct RequestAssembly {
    pub artifacts: Vec<RequestAssemblyArtifact>,
}

#[derive(Debug, Serialize)]
pub struct RequestAssemblyResponse {
    pub id: Uuid,
    pub status_uri: String,
}

pub async fn request_assembly(
    State(state): State<AppState>,
    entity: AuthenticatedEntity,
    Json(request): Json<RequestAssembly>,
) -> Result<Json<RequestAssemblyResponse>, StatusCode> {
    todo!("unimplemented")
}
