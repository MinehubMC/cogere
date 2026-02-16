use crate::{
    auth::{auth::AuthSession, permissions::HasPermissions},
    database::machine_keys::get_machinekey_by_id,
    models::auth::{MachineKey, User},
    server::AppState,
};
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
};
use base64::{Engine as _, engine::general_purpose};
use password_auth::verify_password;
use tokio::task;
use uuid::Uuid;

#[derive(Debug)]
pub enum AuthenticatedEntity {
    User(User),
    Machine(MachineKey),
}

impl HasPermissions for AuthenticatedEntity {
    fn has_permission(&self, permission: crate::auth::permissions::Permission) -> bool {
        match self {
            Self::User(user) => user.has_permission(permission),
            Self::Machine(machine) => machine.has_permission(permission),
        }
    }

    fn identifier(&self) -> String {
        match self {
            Self::User(user) => user.identifier(),
            Self::Machine(machine) => machine.identifier(),
        }
    }
}

impl FromRequestParts<AppState> for AuthenticatedEntity {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        tracing::debug!("AuthenticatedEntity extractor called");

        if let Ok(auth_session) = AuthSession::from_request_parts(parts, state).await {
            tracing::debug!("AuthSession extracted successfully");
            if let Some(user) = auth_session.user().await {
                tracing::info!("Authenticated as user: {}", user.username);
                return Ok(AuthenticatedEntity::User(user));
            } else {
                tracing::debug!("AuthSession exists but no user logged in");
            }
        } else {
            tracing::debug!("No valid user session found");
        }

        tracing::debug!("Attempting machine key authentication");

        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Basic "))
            .ok_or_else(|| {
                tracing::warn!("No Authorization header with Basic scheme found");
                StatusCode::UNAUTHORIZED
            })?;

        tracing::debug!("Found Authorization header");

        let decoded = general_purpose::STANDARD.decode(auth_header).map_err(|e| {
            tracing::error!("Failed to decode base64: {:?}", e);
            StatusCode::UNAUTHORIZED
        })?;

        let decoded = String::from_utf8(decoded).map_err(|e| {
            tracing::error!("Failed to parse UTF-8: {:?}", e);
            StatusCode::UNAUTHORIZED
        })?;

        let (machine_id, password) = decoded.split_once(':').ok_or_else(|| {
            tracing::error!("Invalid Basic auth format (missing colon)");
            StatusCode::UNAUTHORIZED
        })?;

        tracing::debug!("Attempting to authenticate machine with ID: {}", machine_id);

        let machine_uuid = Uuid::parse_str(machine_id).map_err(|e| {
            tracing::error!("Failed to parse machine ID as UUID: {:?}", e);
            StatusCode::UNAUTHORIZED
        })?;

        let machine = get_machinekey_by_id(&state.db, machine_uuid)
            .await
            .map_err(|e| {
                tracing::error!("Database error fetching machine key: {:?}", e);
                StatusCode::UNAUTHORIZED
            })?
            .ok_or_else(|| {
                tracing::warn!("Machine key not found: {}", machine_uuid);
                StatusCode::UNAUTHORIZED
            })?;

        tracing::debug!("Found machine key: {}", machine.description);

        let valid = task::spawn_blocking({
            let key_hash = machine.key_hash.clone();
            let password = password.to_owned();
            move || verify_password(&password, &key_hash).is_ok()
        })
        .await
        .map_err(|e| {
            tracing::error!("Password verification task failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if !valid {
            tracing::warn!("Invalid password for machine key: {}", machine_uuid);
            return Err(StatusCode::UNAUTHORIZED);
        }

        tracing::info!(
            "Successfully authenticated as machine: {}",
            machine.description
        );
        Ok(AuthenticatedEntity::Machine(machine))
    }
}
