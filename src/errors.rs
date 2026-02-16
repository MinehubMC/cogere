use tokio::task;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    TaskJoin(#[from] task::JoinError),

    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
