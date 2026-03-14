use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    assembler::{ArtifactCoordinate, errors::AssemblyError},
    models::plugins::{Plugin, PluginVersion},
};

#[derive(Debug)]
pub enum AssemblyStatus {
    Pending,
    Running,
    Completed {
        blob_id: Uuid,
        expires_at: DateTime<Utc>,
    },
    Failed {
        error: AssemblyError,
    },
}

#[derive(Debug, Clone)]
pub struct ResolvedArtifact {
    pub plugin: Plugin,
    pub version: PluginVersion,
}

#[derive(Debug, Serialize)]
pub struct Assembly {
    pub id: Uuid,
    pub group_id: Uuid,
    pub status: String,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub blob_id: Option<Uuid>,
    pub artifacts: Vec<ArtifactCoordinate>,
}
