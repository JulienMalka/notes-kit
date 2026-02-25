use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum StorageError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

#[derive(Debug, Clone, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("user not found: {0}")]
    UserNotFound(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Error)]
pub enum RepositoryError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<StorageError> for RepositoryError {
    fn from(e: StorageError) -> Self {
        Self::Storage(e.to_string())
    }
}

impl From<AuthError> for RepositoryError {
    fn from(e: AuthError) -> Self {
        Self::Internal(e.to_string())
    }
}
