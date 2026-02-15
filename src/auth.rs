use axum_login::AuthnBackend;
use password_auth::verify_password;
use sqlx::SqlitePool;
use tokio::task;

use crate::{
    database::users::{get_user_by_id, get_user_by_username},
    errors::Error,
    models::auth::{User, UserCredentials},
};

#[derive(Clone, Debug)]
pub struct Backend {
    db: SqlitePool,
}

impl Backend {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }
}

impl AuthnBackend for Backend {
    type User = User;
    type Credentials = UserCredentials;
    type Error = Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = get_user_by_username(&self.db, creds.username).await?;

        task::spawn_blocking(|| {
            Ok(user.filter(|user| verify_password(creds.password, &user.password_hash).is_ok()))
        })
        .await?
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = get_user_by_id(&self.db, user_id).await?;

        Ok(user)
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;
