use axum_login::AuthUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::permissions::InstanceRole;

#[derive(Clone, Deserialize)]
pub struct UserCredentials {
    pub username: String,
    pub password: String,
    pub next: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: InstanceRole,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .field("role", &self.role)
            .field("email", &self.email)
            .finish()
    }
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for User {
    fn from_row(row: &'_ sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let id = uuid::Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::ColumnDecode {
            index: "id".to_string(),
            source: Box::new(e),
        })?;

        Ok(User {
            id,
            username: row.try_get("username")?,
            email: row.try_get("email")?,
            password_hash: row.try_get("password_hash")?,
            role: row.try_get("role")?,
        })
    }
}

impl AuthUser for User {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

#[derive(Debug)]
pub struct MachineKey {
    pub id: Uuid,
    pub description: String,
    pub group_id: Uuid,
    pub key_hash: String,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for MachineKey {
    fn from_row(row: &'_ sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let id = uuid::Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::ColumnDecode {
            index: "id".to_string(),
            source: Box::new(e),
        })?;

        let group_id_str: String = row.try_get("group_id")?;
        let group_id =
            uuid::Uuid::parse_str(&group_id_str).map_err(|e| sqlx::Error::ColumnDecode {
                index: "group_id".to_string(),
                source: Box::new(e),
            })?;

        Ok(MachineKey {
            id,
            group_id,
            description: row.try_get("description")?,
            key_hash: row.try_get("key_hash")?,
        })
    }
}
