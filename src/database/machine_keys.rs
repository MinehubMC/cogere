use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::auth::MachineKey;

struct Found;

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Found {
    fn from_row(_: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Found)
    }
}

pub async fn get_machinekey_by_id(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<MachineKey>, sqlx::Error> {
    sqlx::query_as::<_, MachineKey>("SELECT * FROM machine_keys WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn machine_key_has_specific_permission(
    pool: &SqlitePool,
    key_id: Uuid,
    resource_type: &str,
    resource_id: Uuid,
    action: &str,
) -> Result<bool, sqlx::Error> {
    let key_id = key_id.to_string();
    let resource_id = resource_id.to_string();

    let row = sqlx::query_as::<_, Found>(
        "SELECT 1 FROM machine_key_permissions
             WHERE key_id = ? AND resource_type = ?
               AND resource_id = ? AND action = ?",
    )
    .bind(key_id)
    .bind(resource_type)
    .bind(resource_id)
    .bind(action)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

pub async fn machine_key_has_wide_permission(
    pool: &SqlitePool,
    key_id: Uuid,
    resource_type: &str,
    action: &str,
) -> Result<bool, sqlx::Error> {
    let key_id = key_id.to_string();

    let row = sqlx::query_as::<_, Found>(
        "SELECT 1 FROM machine_key_permissions
         WHERE key_id = ? AND resource_type = ?
           AND resource_id IS NULL AND action = ?",
    )
    .bind(key_id)
    .bind(resource_type)
    .bind(action)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}
