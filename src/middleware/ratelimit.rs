use axum::http::{self, Request};
use base64::{Engine, engine::general_purpose};
use serde::{Deserialize, Serialize};
use tower_governor::{GovernorError, key_extractor::KeyExtractor};
use uuid::Uuid;

use crate::{middleware::client_ip::ClientIp, models::auth::User};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EntityKeyExtractor;

impl KeyExtractor for EntityKeyExtractor {
    type Key = String;

    fn extract<B>(&self, req: &Request<B>) -> Result<Self::Key, GovernorError> {
        if let Some(user) = req.extensions().get::<User>() {
            return Ok(format!("user:{}", user.id));
        }

        if let Some(key_id) = req
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Basic "))
            .and_then(|b64| general_purpose::STANDARD.decode(b64).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|s| s.split_once(':').map(|(id, _)| id.to_string()))
            .and_then(|id| Uuid::parse_str(&id).ok())
        {
            return Ok(format!("machine:{}", key_id));
        }

        if let Some(client_ip) = req.extensions().get::<ClientIp>() {
            return Ok(format!("ip:{}", client_ip.0));
        }

        Err(GovernorError::UnableToExtractKey)
    }
}
