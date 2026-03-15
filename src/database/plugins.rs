use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    database::blobs,
    models::{blobs::BlobEntityType, plugins::PluginVersion},
};

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

    let entity = BlobEntityType::Plugin {
        id: input.plugin_id,
    };

    if input.is_new_blob {
        blobs::create_blob(
            &mut *tx,
            input.group_id,
            input.blob_id,
            entity,
            input.sha256,
            size_bytes,
        )
        .await?;
    } else {
        blobs::add_blob_ref(&mut *tx, input.group_id, input.blob_id, entity).await?;
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

pub async fn get_plugin_version(
    pool: &SqlitePool,
    group_id: Uuid,
    plugin_group_id: String,
    plugin_artifact_id: String,
    version: String,
) -> Result<Option<PluginVersion>, sqlx::Error> {
    let group_id_str = group_id.to_string();

    let row = sqlx::query!(
        r#"
        SELECT
            pv.id AS "id!",
            pv.plugin_id AS "plugin_id!",
            pv.version AS "version!",
            pv.blob_id AS blob_id
        FROM plugin_versions pv
        JOIN plugins p ON p.id  = pv.plugin_id
        JOIN group_plugins gp ON gp.plugin_id = p.id
        WHERE gp.group_id = ?
          AND p.plugin_group_id = ?
          AND p.plugin_artifact_id = ?
          AND pv.version = ?
        "#,
        group_id_str,
        plugin_group_id,
        plugin_artifact_id,
        version,
    )
    .fetch_optional(pool)
    .await?;

    row.map(|r| {
        let id = Uuid::parse_str(&r.id).map_err(|e| sqlx::Error::ColumnDecode {
            index: "id".to_string(),
            source: Box::new(e),
        })?;
        let plugin_id = Uuid::parse_str(&r.plugin_id).map_err(|e| sqlx::Error::ColumnDecode {
            index: "plugin_id".to_string(),
            source: Box::new(e),
        })?;
        let blob_id = r
            .blob_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "blob_id".to_string(),
                source: Box::new(e),
            })?;

        Ok(PluginVersion {
            id,
            plugin_id,
            version: r.version,
            blob_id,
        })
    })
    .transpose()
}
