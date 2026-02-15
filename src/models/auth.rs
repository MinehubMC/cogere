use axum_login::AuthUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Role {
    Guest = 0,
    User = 1,
    Admin = 2,
}

impl Role {
    pub fn from_i64(v: i64) -> Option<Self> {
        match v {
            0 => Some(Self::Guest),
            1 => Some(Self::User),
            2 => Some(Self::Admin),
            _ => None,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Guest => write!(f, "guest"),
            Role::User => write!(f, "user"),
            Role::Admin => write!(f, "admin"),
        }
    }
}

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
    pub role: Role,
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

        let role_int: i64 = row.try_get("role")?;
        let role = Role::from_i64(role_int).ok_or_else(|| sqlx::Error::ColumnDecode {
            index: "role".to_string(),
            source: "invalid role value".into(),
        })?;

        Ok(User {
            id,
            username: row.try_get("username")?,
            email: row.try_get("email")?,
            password_hash: row.try_get("password_hash")?,
            role,
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

        Ok(MachineKey {
            id,
            description: row.try_get("description")?,
            group_id: row.try_get("group_id")?,
            key_hash: row.try_get("key_hash")?,
        })
    }
}
