use uuid::Uuid;

use crate::auth::permissions::GroupRole;

#[derive(Debug)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub quota_bytes: u64,
    pub quota_used_bytes: u64,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Group {
    fn from_row(row: &'_ sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let id = uuid::Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::ColumnDecode {
            index: "id".to_string(),
            source: Box::new(e),
        })?;

        Ok(Group {
            id,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            quota_bytes: row.try_get("quota_bytes")?,
            quota_used_bytes: row.try_get("used_bytes")?,
        })
    }
}

impl Group {
    pub fn quota_would_exceed(&self, additional: u64) -> bool {
        self.quota_bytes != 0 && self.quota_used_bytes + additional > self.quota_bytes
    }

    /// returns remaining bytes, or None if unlimited.
    pub fn quota_available_bytes(&self) -> Option<u64> {
        (self.quota_bytes != 0).then(|| self.quota_bytes.saturating_sub(self.quota_used_bytes))
    }
}

#[derive(Debug)]
pub struct GroupMember {
    pub id: Uuid,
    pub username: String,
    pub role: GroupRole,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for GroupMember {
    fn from_row(row: &'_ sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let id = uuid::Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::ColumnDecode {
            index: "id".to_string(),
            source: Box::new(e),
        })?;

        Ok(GroupMember {
            id,
            username: row.try_get("username")?,
            role: row.try_get("group_role")?,
        })
    }
}
