use crate::{
    auth::{auth::AuthSession, permissions::InstanceRole},
    errors::{AppError, Error},
};
use axum::{extract::Request, middleware::Next, response::Response};

pub async fn require_admin(
    auth: AuthSession,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    match auth.user().await {
        Some(user) if user.role == InstanceRole::InstanceAdmin => Ok(next.run(req).await),
        _ => Err(Error::Forbidden.into()),
    }
}
