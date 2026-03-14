use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tokio::task;

use crate::{assembler::errors::AssemblyError, storage::StorageError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    TaskJoin(#[from] task::JoinError),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
    #[error("forbidden")]
    Forbidden,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Template(#[from] askama::Error),
    #[error("unauthorized")]
    Unauthorized,
    #[error("not allowed: {0}")]
    NotAllowed(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal server error: {0}")]
    Internal(String),
    #[error(transparent)]
    Assembly(#[from] AssemblyError),
}

pub struct AppError(Error);

impl From<Error> for AppError {
    fn from(e: Error) -> Self {
        AppError(e)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError(Error::Sqlx(e))
    }
}

impl From<StorageError> for AppError {
    fn from(e: StorageError) -> Self {
        AppError(Error::Storage(e))
    }
}

impl From<askama::Error> for AppError {
    fn from(e: askama::Error) -> Self {
        AppError(Error::Template(e))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::BadRequest(msg) => {
                return (StatusCode::BAD_REQUEST, msg.clone()).into_response();
            }
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::Sqlx(e) => {
                tracing::error!("database error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Error::TaskJoin(e) => {
                tracing::error!("task join error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Error::Storage(e) => {
                tracing::error!("storage error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Error::Template(e) => {
                tracing::error!("template error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Error::NotAllowed(msg) => {
                return (StatusCode::FORBIDDEN, msg.clone()).into_response();
            }
            Error::NotFound(msg) => {
                return (StatusCode::NOT_FOUND, msg.clone()).into_response();
            }
            Error::Conflict(msg) => {
                return (StatusCode::CONFLICT, msg.clone()).into_response();
            }
            Error::Internal(e) => {
                tracing::error!("internal error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Error::Assembly(e) => {
                let inner = match e {
                    AssemblyError::ArtifactNotFound(msg) => Error::NotFound(msg.clone()),
                    AssemblyError::UnsupportedProvider(msg) => Error::BadRequest(msg.clone()),
                    AssemblyError::ExternalFetch(msg) => Error::Internal(msg.clone()),
                    AssemblyError::QueueUnavailable => {
                        Error::Internal("assembly queue unavailable".into())
                    }
                    AssemblyError::Timeout(_) => Error::Internal("assembly timed out".into()),
                    AssemblyError::Sqlx(e) => Error::Sqlx(e),
                    AssemblyError::Storage(e) => Error::Storage(e),
                    AssemblyError::Zip(e) => Error::Internal(e.to_string()),
                    AssemblyError::Io(e) => Error::Internal(e.to_string()),
                    AssemblyError::Internal(e) => Error::Internal(e.to_string()),
                    AssemblyError::NoArtifacts => {
                        Error::BadRequest("no artifacts provided".to_string())
                    }
                };
                return AppError(inner).into_response();
            }
        };
        status.into_response()
    }
}
