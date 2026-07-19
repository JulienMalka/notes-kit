use crate::error::StorageError;
use async_trait::async_trait;

/// A partial read of a file: the requested byte window plus the file's
/// total size (needed for HTTP `Content-Range` responses). `bytes` is
/// empty when the requested start lies at or beyond the end of the file.
pub struct RangeRead {
    pub bytes: Vec<u8>,
    pub total_size: u64,
}

#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    async fn list_files(&self, extension: &str) -> Result<Vec<String>, StorageError>;

    async fn read_file(&self, path: &str) -> Result<String, StorageError>;

    fn is_path_safe(&self, path: &str) -> bool;

    async fn read_file_bytes(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        self.read_file(path).await.map(|s| s.into_bytes())
    }

    /// Read up to `max_len` bytes starting at `start`. The default reads
    /// the whole file and slices — backends serving large media (video)
    /// should override with a real ranged read.
    async fn read_file_range(
        &self,
        path: &str,
        start: u64,
        max_len: u64,
    ) -> Result<RangeRead, StorageError> {
        let bytes = self.read_file_bytes(path).await?;
        let total_size = bytes.len() as u64;
        let start = start.min(total_size);
        let end = start.saturating_add(max_len).min(total_size);
        Ok(RangeRead {
            bytes: bytes[start as usize..end as usize].to_vec(),
            total_size,
        })
    }

    async fn list_all_files(&self) -> Result<Vec<String>, StorageError> {
        Ok(Vec::new())
    }

    async fn listing_hash(&self, extension: &str) -> Result<Option<u64>, StorageError> {
        let _ = extension;
        Ok(None)
    }
}
