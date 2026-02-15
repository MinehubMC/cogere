#[derive(Debug)]
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

#[derive(Debug)]
pub struct APIKey {
    pub id: uuid::Uuid,
    pub description: String,
    pub role: Role,
    pub hashed_key: String,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for APIKey {
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

        Ok(APIKey {
            id,
            description: row.try_get("description")?,
            hashed_key: row.try_get("hashed_key")?,
            role,
        })
    }
}
