use std::sync::Arc;

use chrono::Duration;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::{
    database::assembly::{cleanup_expired_assemblies, cleanup_old_assemblies},
    models::settings::InstanceSettings,
    storage::filesystem::FilesystemStorage,
};

pub async fn run(
    pool: SqlitePool,
    storage: FilesystemStorage,
    settings: Arc<RwLock<InstanceSettings>>,
) {
    loop {
        let (interval_secs, max_age_days) = {
            let s = settings.read().await;
            (
                s.assembly_cleanup_interval_secs,
                s.assembly_max_age_days as i64,
            )
        };

        if let Err(e) = cleanup_expired_assemblies(&pool, &storage).await {
            tracing::error!(error = %e, "assembly cleanup failed");
        }

        if let Err(e) = cleanup_old_assemblies(&pool, Duration::days(max_age_days)).await {
            tracing::error!(error = %e, "old assembly cleanup failed");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
    }
}
