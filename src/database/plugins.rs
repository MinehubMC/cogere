use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn create_plugin(
    pool: &SqlitePool,
    id: Uuid,
    artifact_id: &str,
    group_id: &str,
    version: &str,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO plugins (id, artifact_id, group_id, version)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(artifact_id)
    .bind(group_id)
    .bind(version)
    .execute(pool)
    .await;

    match &result {
        Ok(_) => tracing::debug!("Plugin created successfully"),
        Err(e) => tracing::error!("Failed to create plugin: {:?}", e),
    }

    Ok(())
}
