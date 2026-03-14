use crate::{
    assembler::{self, ArtifactCoordinate},
    auth::{
        extractor::AuthenticatedEntity,
        permissions::{Action, PermissionCheck, ResourceType, check::PermissionChecker},
    },
    database,
    errors::{AppError, Error},
    server::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, de};
use uuid::Uuid;

impl<'de> de::Deserialize<'de> for ArtifactCoordinate {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.splitn(3, ':').collect();
        match parts.as_slice() {
            [a, b, c] => Ok(ArtifactCoordinate {
                group_id: a.to_string(),
                artifact_id: b.to_string(),
                version: c.to_string(),
            }),
            _ => Err(de::Error::custom("expected group_id:artifact_id:version")),
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct RequestAssembly {
    pub artifacts: Vec<ArtifactCoordinate>,
}

pub async fn request_assembly(
    State(state): State<AppState>,
    entity: AuthenticatedEntity,
    Path(group_id): Path<Uuid>,
    Json(request): Json<RequestAssembly>,
) -> Result<Response, AppError> {
    match assembler::request_assembly(&state, &entity, group_id, request.artifacts).await {
        Ok(data) => Ok((StatusCode::CREATED, Json(data)).into_response()),
        Err(e) => Err(e.into()),
    }
}

pub async fn get_assembly(
    State(state): State<AppState>,
    entity: AuthenticatedEntity,
    Path((group_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Response, AppError> {
    PermissionChecker::new(&state.db, &entity)
        .require(PermissionCheck::on_type(ResourceType::Artifact, Action::Get).in_group(group_id))
        .await?;

    match database::assembly::get_assembly(&state.db, group_id, id).await {
        Ok(Some(assembly)) => Ok(Json(assembly).into_response()),
        Ok(None) => Err(Error::NotFound(format!("assembly {id} not found")).into()),
        Err(e) => Err(e.into()),
    }
}
