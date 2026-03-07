use uuid::Uuid;

pub mod auth;
pub mod settings;

#[derive(Debug)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
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
        })
    }
}
