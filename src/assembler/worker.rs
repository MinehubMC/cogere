use std::{
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use bytes::Bytes;
use chrono::Utc;
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{
    assembler::{errors::AssemblyError, job::AssemblyJob},
    database::{
        assembly::{get_artifact, set_assembly_status},
        blobs::create_blob,
    },
    models::{assembly::AssemblyStatus, plugins::PluginSource, settings::InstanceSettings},
    storage::{LocalStorage, StorageError, filesystem::FilesystemStorage},
};

pub async fn run(
    mut rx: mpsc::Receiver<AssemblyJob>,
    pool: SqlitePool,
    settings: Arc<RwLock<InstanceSettings>>,
    storage: FilesystemStorage,
    active_jobs: Arc<AtomicUsize>,
) {
    while let Some(job) = rx.recv().await {
        let pool = pool.clone();
        let settings = settings.clone();
        let storage = storage.clone();
        let active = active_jobs.clone();

        tokio::spawn(async move {
            active.fetch_add(1, Ordering::Relaxed);
            tracing::info!(
                "assembly job started, active workers: {}",
                active.load(Ordering::Relaxed)
            );

            let (timeout_secs, expires_secs) = {
                let s = settings.read().await;
                (s.assembly_timeout_secs, s.assembly_expiry_secs)
            };

            let result = tokio::time::timeout(
                Duration::from_secs(timeout_secs),
                process(job.clone(), &pool, &storage),
            )
            .await;

            let status = match result {
                Ok(Ok(blob_id)) => {
                    tracing::info!(assembly_id = %job.id, "assembly completed");
                    let expires_at = Utc::now() + chrono::Duration::seconds(expires_secs as i64);

                    AssemblyStatus::Completed {
                        blob_id,
                        expires_at,
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!(assembly_id = %job.id, error = %e, "assembly failed");
                    AssemblyStatus::Failed { error: e }
                }
                Err(_) => {
                    tracing::error!(assembly_id = %job.id, timeout_secs, "assembly timed out");
                    AssemblyStatus::Failed {
                        error: AssemblyError::Timeout(timeout_secs),
                    }
                }
            };

            if let Err(e) = set_assembly_status(&pool, job.id, status).await {
                tracing::error!(assembly_id = %job.id, error = %e, "failed to update assembly status");
            }

            active.fetch_sub(1, Ordering::Relaxed);
            tracing::info!(
                assembly_id = %job.id,
                active = active.load(Ordering::Relaxed),
                "assembly job finished"
            );
        });
    }
}

async fn process(
    job: AssemblyJob,
    pool: &SqlitePool,
    storage: &FilesystemStorage,
) -> Result<Uuid, AssemblyError> {
    set_assembly_status(pool, job.id, AssemblyStatus::Running).await?;

    let mut zip_buf = Vec::new();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for coord in &job.artifacts {
        let resolved = get_artifact(pool, coord, job.group_id).await?;

        let data = if resolved.version.is_cached() {
            let blob_id = resolved.version.blob_id.unwrap();
            storage.get(blob_id).await.map_err(|e| match e {
                StorageError::NotFound(_) => AssemblyError::ArtifactNotFound(blob_id.to_string()),
                e => AssemblyError::Storage(e),
            })?
        } else {
            fetch_external(&resolved.plugin.source, &coord.version).await?
        };

        let filename = format!(
            "{}.{}-{}.jar",
            coord.group_id, coord.artifact_id, coord.version
        );
        zip.start_file(filename, options)?;
        zip.write_all(&data)?;
    }

    zip.finish()?;

    let zip_bytes = Bytes::from(zip_buf);
    let sha256 = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&zip_bytes);
        format!("{:x}", hasher.finalize())
    };

    let blob_id = Uuid::now_v7();
    let size_bytes = zip_bytes.len() as i64;

    create_blob(pool, blob_id, sha256, size_bytes).await?;

    storage.put(blob_id, zip_bytes).await?;

    Ok(blob_id)
}

async fn fetch_external(source: &PluginSource, _version: &str) -> Result<Bytes, AssemblyError> {
    match source {
        PluginSource::External {
            provider,
            external_id: _,
        } => match provider.as_str() {
            p => Err(AssemblyError::UnsupportedProvider(p.to_string())),
        },
        PluginSource::Local => unreachable!("local plugins must be cached"),
    }
}
