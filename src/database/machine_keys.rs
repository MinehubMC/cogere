use sqlx::SqlitePool;

use crate::models::auth::MachineKey;

pub async fn get_all_machinekeys(pool: &SqlitePool) -> Result<Vec<MachineKey>, sqlx::Error> {
    sqlx::query_as::<_, MachineKey>("SELECT * FROM machine_keys")
        .fetch_all(pool)
        .await
}
