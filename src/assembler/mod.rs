use serde::Serialize;
use uuid::Uuid;

use crate::{
    assembler::{errors::AssemblyError, job::AssemblyJob},
    auth::{
        extractor::AuthenticatedEntity,
        permissions::{Action, PermissionCheck, ResourceType, check::PermissionChecker},
    },
    database,
    errors::Error,
    server::AppState,
};

pub mod errors;
pub mod job;
pub mod worker;

#[derive(Clone, Debug, Serialize)]
pub struct ArtifactCoordinate {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct RequestAssemblyResponse {
    pub id: Uuid,
    pub status_uri: String,
}

pub async fn request_assembly(
    state: &AppState,
    entity: &AuthenticatedEntity,
    group_id: Uuid,
    artifacts: Vec<ArtifactCoordinate>,
) -> Result<RequestAssemblyResponse, Error> {
    PermissionChecker::new(&state.db, &entity)
        .require(PermissionCheck::on_type(ResourceType::Artifact, Action::Get).in_group(group_id))
        .await?;

    if artifacts.len() == 0 {
        return Err(AssemblyError::NoArtifacts.into());
    }

    // TODO CHECK FOR PER ARTIFACT PERMISSION

    let id = database::assembly::create_assembly(&state.db, group_id, artifacts.clone()).await?;

    state
        .assembly_tx
        .send(AssemblyJob {
            id,
            group_id,
            artifacts,
        })
        .await
        .map_err(|_| AssemblyError::QueueUnavailable)?;

    let path = format!("/api/v1/groups/{group_id}/assemblies/{id}");
    let status_uri = state
        .config
        .public_base_url
        .join(&path)
        .map_err(|_| Error::Internal("url join failed".into()))?
        .to_string();

    Ok(RequestAssemblyResponse { id, status_uri })
}
