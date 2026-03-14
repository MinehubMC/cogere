use sqlx::SqlitePool;
use uuid::Uuid;

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

pub async fn create_blob(
    pool: &SqlitePool,
    blob_id: Uuid,
    sha256: String,
    size_bytes: i64,
) -> Result<(), sqlx::Error> {
    let blob_id_str = blob_id.to_string();

    sqlx::query!(
        "INSERT INTO blobs (id, sha256, size_bytes) VALUES (?, ?, ?)",
        blob_id_str,
        sha256,
        size_bytes
    )
    .execute(pool)
    .await?;

    Ok(())
}
