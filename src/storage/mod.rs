use bytes::Bytes;
use uuid::Uuid;

pub mod filesystem;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("object not found: {0}")]
    NotFound(Uuid),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

#[trait_variant::make(Storage: Send)]
pub trait LocalStorage {
    async fn put(&self, key: Uuid, data: Bytes) -> Result<(), StorageError>;
    async fn get(&self, key: Uuid) -> Result<Bytes, StorageError>;
    async fn delete(&self, key: Uuid) -> Result<(), StorageError>;
    async fn exists(&self, key: Uuid) -> Result<bool, StorageError>;
}
