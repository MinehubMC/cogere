use sqlx::SqlitePool;

use crate::models::auth::APIKey;

pub async fn get_all_apikeys(pool: &SqlitePool) -> Result<Vec<APIKey>, sqlx::Error> {
    sqlx::query_as::<_, APIKey>("SELECT * FROM api_keys")
        .fetch_all(pool)
        .await
}
