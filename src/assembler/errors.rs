use serde::Serialize;

use crate::storage::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum AssemblyError {
    #[error("artifact not found: {0}")]
    ArtifactNotFound(String),
    #[error("assembly timed out after {0}s")]
    Timeout(u64),
    #[error("queue unavailable")]
    QueueUnavailable,
    #[error("unsupported external provider: {0}")]
    UnsupportedProvider(String),
    #[error("failed to fetch external artifact: {0}")]
    ExternalFetch(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("no artifacts provided")]
    NoArtifacts,
}

impl Serialize for AssemblyError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
