use sqlx::SqlitePool;
use uuid::Uuid;

pub struct CreateLocalPluginOptions {
    pub plugin_id: Uuid,
    pub version_id: Uuid,
    pub blob_id: Uuid,
    pub group_id: Uuid,
    pub plugin_group_id: String,
    pub plugin_artifact_id: String,
    pub version: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub is_new_blob: bool,
}

pub async fn create_local_plugin(
    db: &SqlitePool,
    input: CreateLocalPluginOptions,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;

    let plugin_id = input.plugin_id.to_string();
    let version_id = input.version_id.to_string();
    let blob_id = input.blob_id.to_string();
    let group_id = input.group_id.to_string();
    let size_bytes = input.size_bytes as i64;

    if input.is_new_blob {
        sqlx::query!(
            "INSERT INTO blobs (id, sha256, size_bytes, ref_count)
             VALUES (?, ?, ?, 1)",
            blob_id,
            input.sha256,
            size_bytes,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE groups SET used_bytes = used_bytes + ? WHERE id = ?",
            size_bytes,
            group_id,
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            "UPDATE blobs SET ref_count = ref_count + 1 WHERE id = ?",
            blob_id,
        )
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query!(
        "INSERT INTO plugins (id, plugin_group_id, plugin_artifact_id, source)
         VALUES (?, ?, ?, 'local')",
        plugin_id,
        input.plugin_group_id,
        input.plugin_artifact_id,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO plugin_versions (id, plugin_id, version, blob_id)
         VALUES (?, ?, ?, ?)",
        version_id,
        plugin_id,
        input.version,
        blob_id,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO group_plugins (group_id, plugin_id, is_owner, visibility)
         VALUES (?, ?, 1, 'private')",
        group_id,
        plugin_id,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
