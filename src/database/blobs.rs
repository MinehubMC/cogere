use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    errors::Error,
    models::{blobs::BlobEntityType, plugins::Blob},
    storage::{LocalStorage, filesystem::FilesystemStorage},
};

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
    executor: &mut sqlx::SqliteConnection,
    group_id: Uuid,
    blob_id: Uuid,
    entity: BlobEntityType,
    sha256: String,
    size_bytes: i64,
) -> Result<(), sqlx::Error> {
    let group_id_str = group_id.to_string();
    let blob_id_str = blob_id.to_string();
    let entity_id_str = entity.id().to_string();
    let entity_type_str = entity.as_type_str();

    sqlx::query!(
        "INSERT INTO blobs (id, sha256, size_bytes) VALUES (?, ?, ?)",
        blob_id_str,
        sha256,
        size_bytes,
    )
    .execute(&mut *executor)
    .await?;

    sqlx::query!(
        "INSERT INTO blob_refs (blob_id, group_id, entity_type, entity_id)
         VALUES (?, ?, ?, ?)",
        blob_id_str,
        group_id_str,
        entity_type_str,
        entity_id_str,
    )
    .execute(&mut *executor)
    .await?;

    sqlx::query!(
        "UPDATE groups SET used_bytes = used_bytes + ? WHERE id = ?",
        size_bytes,
        group_id_str,
    )
    .execute(&mut *executor)
    .await?;

    Ok(())
}

pub async fn add_blob_ref(
    executor: &mut sqlx::SqliteConnection,
    blob_id: Uuid,
    group_id: Uuid,
    entity: BlobEntityType,
) -> Result<(), sqlx::Error> {
    let blob_id_str = blob_id.to_string();
    let group_id_str = group_id.to_string();
    let entity_id_str = entity.id().to_string();
    let entity_type_str = entity.as_type_str();

    let size_bytes = sqlx::query_scalar!("SELECT size_bytes FROM blobs WHERE id = ?", blob_id_str)
        .fetch_one(&mut *executor)
        .await?;

    sqlx::query!(
        "INSERT INTO blob_refs (blob_id, group_id, entity_type, entity_id)
         VALUES (?, ?, ?, ?)",
        blob_id_str,
        group_id_str,
        entity_type_str,
        entity_id_str,
    )
    .execute(&mut *executor)
    .await?;

    sqlx::query!(
        "UPDATE groups SET used_bytes = used_bytes + ? WHERE id = ?",
        size_bytes,
        group_id_str,
    )
    .execute(&mut *executor)
    .await?;

    Ok(())
}

pub async fn remove_blob_ref(
    pool: &SqlitePool,
    storage: &FilesystemStorage,
    blob_id: Uuid,
    group_id: Uuid,
    entity: BlobEntityType,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    let blob_id_str = blob_id.to_string();
    let group_id_str = group_id.to_string();
    let entity_id_str = entity.id().to_string();

    let size_bytes = sqlx::query_scalar!("SELECT size_bytes FROM blobs WHERE id = ?", blob_id_str,)
        .fetch_one(&mut *tx)
        .await?;

    sqlx::query!(
        "DELETE FROM blob_refs WHERE blob_id = ? AND entity_id = ?",
        blob_id_str,
        entity_id_str,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE groups SET used_bytes = used_bytes - ? WHERE id = ?",
        size_bytes,
        group_id_str,
    )
    .execute(&mut *tx)
    .await?;

    let remaining = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM blob_refs WHERE blob_id = ?",
        blob_id_str,
    )
    .fetch_one(&mut *tx)
    .await?;

    if remaining == 0 {
        sqlx::query!("DELETE FROM blobs WHERE id = ?", blob_id_str)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    if remaining == 0 {
        storage.delete(blob_id).await?;
    }

    Ok(())
}
