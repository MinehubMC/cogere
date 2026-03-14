use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    assembler::{ArtifactCoordinate, errors::AssemblyError},
    models::{
        assembly::{Assembly, AssemblyStatus, ResolvedArtifact},
        plugins::{Plugin, PluginSource, PluginVersion},
    },
};

pub async fn create_assembly(
    pool: &SqlitePool,
    group_id: Uuid,
    artifacts: Vec<ArtifactCoordinate>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();

    let mut tx = pool.begin().await?;

    let id_str = id.to_string();
    let group_id_str = group_id.to_string();

    sqlx::query!(
        "INSERT INTO assemblies (id, group_id)
         VALUES (?, ?)",
        id_str,
        group_id_str,
    )
    .execute(&mut *tx)
    .await?;

    for artifact in artifacts {
        sqlx::query!(
            "INSERT INTO assembly_artifacts (assembly_id, group_id, artifact_id, version)
             VALUES (?, ?, ?, ?)",
            id_str,
            artifact.group_id,
            artifact.artifact_id,
            artifact.version
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(id)
}

pub async fn set_assembly_status(
    pool: &SqlitePool,
    assembly_id: Uuid,
    status: AssemblyStatus,
) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    let assembly_id = assembly_id.to_string();

    match status {
        AssemblyStatus::Pending => {
            sqlx::query!(
                "UPDATE assemblies SET status = 'pending', updated_at = ? WHERE id = ?",
                now,
                assembly_id
            )
            .execute(pool)
            .await?;
        }
        AssemblyStatus::Running => {
            sqlx::query!(
                "UPDATE assemblies SET status = 'running', started_at = ?, updated_at = ? WHERE id = ?",
                now, now, assembly_id
            )
            .execute(pool)
            .await?;
        }
        AssemblyStatus::Completed {
            blob_id,
            expires_at,
        } => {
            let blob_id = blob_id.to_string();
            sqlx::query!(
                "UPDATE assemblies SET status = 'completed', completed_at = ?, updated_at = ?, blob_id = ?, expires_at = ? WHERE id = ?",
                now, now, blob_id, expires_at, assembly_id
            )
            .execute(pool)
            .await?;
        }
        AssemblyStatus::Failed { error } => {
            let error_msg = error.to_string();
            sqlx::query!(
                "UPDATE assemblies SET status = 'failed', completed_at = ?, updated_at = ?, error = ? WHERE id = ?",
                now, now, error_msg, assembly_id
            )
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

pub async fn get_artifact(
    pool: &SqlitePool,
    coord: &ArtifactCoordinate,
    group_id: Uuid,
) -> Result<ResolvedArtifact, AssemblyError> {
    let group_id_str = group_id.to_string();

    let row = sqlx::query!(
        r#"
        SELECT
            p.id as plugin_id,
            p.plugin_group_id,
            p.plugin_artifact_id,
            p.source,
            p.external_provider,
            p.external_id,
            pv.id as version_id,
            pv.version,
            pv.blob_id
        FROM plugins p
        JOIN plugin_versions pv ON pv.plugin_id = p.id
        JOIN group_plugins gp ON gp.plugin_id = p.id
        WHERE p.plugin_group_id = ?
          AND p.plugin_artifact_id = ?
          AND pv.version = ?
          AND gp.group_id = ?
        "#,
        coord.group_id,
        coord.artifact_id,
        coord.version,
        group_id_str,
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AssemblyError::ArtifactNotFound(format!(
            "{}:{}:{}",
            coord.group_id, coord.artifact_id, coord.version
        ))
    })?;

    let plugin_id = row
        .plugin_id
        .as_deref()
        .ok_or_else(|| AssemblyError::Internal("plugin id missing".into()))
        .and_then(|s| Uuid::try_parse(s).map_err(|e| AssemblyError::Internal(e.to_string())))?;
    let version_id = row
        .version_id
        .as_deref()
        .ok_or_else(|| AssemblyError::Internal("version id missing".into()))
        .and_then(|s| Uuid::try_parse(s).map_err(|e| AssemblyError::Internal(e.to_string())))?;
    let blob_id = row
        .blob_id
        .as_deref()
        .map(Uuid::try_parse)
        .transpose()
        .map_err(|e| AssemblyError::Internal(e.to_string()))?;

    let source = match row.source.as_str() {
        "local" => PluginSource::Local,
        _ => match (row.external_provider, row.external_id) {
            (Some(provider), Some(external_id)) => PluginSource::External {
                provider,
                external_id,
            },
            _ => {
                return Err(AssemblyError::Internal(format!(
                    "plugin {} has invalid source",
                    plugin_id
                )));
            }
        },
    };

    Ok(ResolvedArtifact {
        plugin: Plugin {
            id: plugin_id,
            plugin_group_id: row.plugin_group_id,
            plugin_artifact_id: row.plugin_artifact_id,
            source,
        },
        version: PluginVersion {
            id: version_id,
            plugin_id,
            version: row.version,
            blob_id,
        },
    })
}

pub async fn get_assembly(
    pool: &SqlitePool,
    group_id: Uuid,
    assembly_id: Uuid,
) -> Result<Option<Assembly>, sqlx::Error> {
    let group_id_str = group_id.to_string();
    let assembly_id_str = assembly_id.to_string();

    let row = sqlx::query!(
        r#"
        SELECT id, group_id, status, updated_at, started_at, completed_at, expires_at, error, blob_id
        FROM assemblies
        WHERE id = ? AND group_id = ?
        "#,
        assembly_id_str,
        group_id_str,
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let artifact_rows = sqlx::query!(
        r#"
        SELECT group_id, artifact_id, version
        FROM assembly_artifacts
        WHERE assembly_id = ?
        "#,
        assembly_id_str
    )
    .fetch_all(pool)
    .await?;

    let artifacts = artifact_rows
        .into_iter()
        .map(|r| ArtifactCoordinate {
            group_id: r.group_id,
            artifact_id: r.artifact_id,
            version: r.version,
        })
        .collect();

    let id = Uuid::parse_str(&row.id).map_err(|e| sqlx::Error::ColumnDecode {
        index: "id".into(),
        source: Box::new(e),
    })?;
    let group_id = Uuid::parse_str(&row.group_id).map_err(|e| sqlx::Error::ColumnDecode {
        index: "group_id".into(),
        source: Box::new(e),
    })?;
    let blob_id = row
        .blob_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| sqlx::Error::ColumnDecode {
            index: "blob_id".into(),
            source: Box::new(e),
        })?;

    Ok(Some(Assembly {
        id,
        group_id,
        status: row.status,
        updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "updated_at".into(),
                source: Box::new(e),
            })?,
        started_at: row
            .started_at
            .as_deref()
            .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "started_at".into(),
                source: Box::new(e),
            })?,
        completed_at: row
            .completed_at
            .as_deref()
            .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "completed_at".into(),
                source: Box::new(e),
            })?,
        expires_at: row
            .expires_at
            .as_deref()
            .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "expires_at".into(),
                source: Box::new(e),
            })?,
        error: row.error,
        blob_id,
        artifacts,
    }))
}

pub async fn get_assembly_status(
    pool: &SqlitePool,
    assembly_id: Uuid,
) -> Result<Option<AssemblyStatus>, sqlx::Error> {
    let assembly_id_str = assembly_id.to_string();

    let row = sqlx::query!(
        "SELECT status, blob_id, expires_at, error FROM assemblies WHERE id = ?",
        assembly_id_str
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let status = match row.status.as_str() {
        "pending" => AssemblyStatus::Pending,
        "running" => AssemblyStatus::Running,
        "completed" => AssemblyStatus::Completed {
            blob_id: row
                .blob_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or_else(|| sqlx::Error::ColumnDecode {
                    index: "blob_id".into(),
                    source: "completed assembly missing blob_id".into(),
                })?,
            expires_at: row
                .expires_at
                .as_deref()
                .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()
                .map_err(|e| sqlx::Error::ColumnDecode {
                    index: "expires_at".into(),
                    source: Box::new(e),
                })?
                .ok_or_else(|| sqlx::Error::ColumnDecode {
                    index: "expires_at".into(),
                    source: "completed assembly missing expires_at".into(),
                })?,
        },
        "failed" => AssemblyStatus::Failed {
            error: AssemblyError::Internal(row.error.unwrap_or_else(|| "unknown error".into())),
        },
        s => {
            return Err(sqlx::Error::ColumnDecode {
                index: "status".into(),
                source: format!("unknown status: {s}").into(),
            });
        }
    };

    Ok(Some(status))
}
