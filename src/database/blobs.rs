use sqlx::SqlitePool;

use crate::models::plugins::Blob;

pub async fn find_by_sha256(
    pool: &SqlitePool,
    sha256: String,
) -> Result<Option<Blob>, sqlx::Error> {
    sqlx::query_as::<_, Blob>("SELECT * FROM blobs WHERE sha256 = ?")
        .bind(sha256)
        .fetch_optional(pool)
        .await
}
