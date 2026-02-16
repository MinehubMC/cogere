use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::auth::MachineKey;

pub async fn get_all_machinekeys(pool: &SqlitePool) -> Result<Vec<MachineKey>, sqlx::Error> {
    sqlx::query_as::<_, MachineKey>("SELECT * FROM machine_keys")
        .fetch_all(pool)
        .await
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
