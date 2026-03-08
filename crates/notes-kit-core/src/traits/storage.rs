use crate::error::StorageError;
use async_trait::async_trait;

#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    async fn list_files(&self, extension: &str) -> Result<Vec<String>, StorageError>;

    async fn read_file(&self, path: &str) -> Result<String, StorageError>;

    fn is_path_safe(&self, path: &str) -> bool;

    async fn listing_hash(&self, extension: &str) -> Result<Option<u64>, StorageError> {
        let _ = extension;
        Ok(None)
    }
}
