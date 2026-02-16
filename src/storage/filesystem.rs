use std::path::PathBuf;

use bytes::Bytes;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};
use uuid::Uuid;

use crate::storage::{LocalStorage, StorageError};

#[derive(Clone, Debug)]
pub struct FilesystemStorage {
    root: PathBuf,
}

impl FilesystemStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path_for(&self, key: Uuid) -> PathBuf {
        self.root.join(key.to_string())
    }
}

impl LocalStorage for FilesystemStorage {
    async fn put(&self, key: Uuid, data: Bytes) -> Result<(), StorageError> {
        let mut file = fs::File::create(self.path_for(key)).await?;
        file.write_all(&data).await?;
        file.flush().await?;
        Ok(())
    }
    async fn get(&self, key: Uuid) -> Result<Bytes, StorageError> {
        let mut file = fs::File::open(self.path_for(key)).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(key)
            } else {
                StorageError::Io(e)
            }
        })?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await?;
        Ok(Bytes::from(buf))
    }
    async fn delete(&self, key: Uuid) -> Result<(), StorageError> {
        fs::remove_file(self.path_for(key)).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(key)
            } else {
                StorageError::Io(e)
            }
        })
    }
    async fn exists(&self, key: Uuid) -> Result<bool, StorageError> {
        match fs::metadata(self.path_for(key)).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(StorageError::Io(e)),
        }
    }
}
