use async_trait::async_trait;
use notes_kit_core::error::StorageError;
use notes_kit_core::traits::StorageBackend;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct LocalStorageBackend {
    root: PathBuf,
}

impl LocalStorageBackend {
    pub fn new(root: PathBuf) -> Result<Self, StorageError> {
        if !root.exists() {
            return Err(StorageError::NotFound(format!(
                "directory does not exist: {}",
                root.display()
            )));
        }
        if !root.is_dir() {
            return Err(StorageError::InvalidPath(format!(
                "not a directory: {}",
                root.display()
            )));
        }
        let root = root
            .canonicalize()
            .map_err(|e| StorageError::Io(format!("canonicalize failed: {e}")))?;
        Ok(Self { root })
    }

    fn absolute(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn collect_files<'a>(
        &'a self,
        dir: &'a Path,
        ext: &'a str,
        out: &'a mut Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), StorageError>> + Send + 'a>>
    {
        Box::pin(async move {
            let mut entries = fs::read_dir(dir)
                .await
                .map_err(|e| StorageError::Io(format!("read_dir: {e}")))?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| StorageError::Io(format!("next_entry: {e}")))?
            {
                let path = entry.path();
                let meta = entry
                    .metadata()
                    .await
                    .map_err(|e| StorageError::Io(format!("metadata: {e}")))?;

                if meta.is_dir() {
                    self.collect_files(&path, ext, out).await?;
                } else if path.extension().and_then(|s| s.to_str()) == Some(ext) {
                    if let Ok(relative) = path.strip_prefix(&self.root) {
                        if let Some(s) = relative.to_str() {
                            out.push(s.to_string());
                        }
                    }
                }
            }
            Ok(())
        })
    }
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn list_files(&self, extension: &str) -> Result<Vec<String>, StorageError> {
        let mut files = Vec::new();
        self.collect_files(&self.root, extension, &mut files).await?;
        Ok(files)
    }

    async fn read_file(&self, path: &str) -> Result<String, StorageError> {
        let abs = self.absolute(path);
        fs::read_to_string(&abs).await.map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                StorageError::NotFound(format!("file not found: {path}"))
            }
            std::io::ErrorKind::PermissionDenied => {
                StorageError::PermissionDenied(format!("permission denied: {path}"))
            }
            _ => StorageError::Io(format!("read error: {e}")),
        })
    }

    fn is_path_safe(&self, path: &str) -> bool {
        if path.contains("..") {
            return false;
        }
        let abs = self.absolute(path);
        match abs.canonicalize() {
            Ok(canonical) => canonical.starts_with(&self.root),
            Err(_) => {
                abs.parent()
                    .and_then(|p| p.canonicalize().ok())
                    .is_some_and(|cp| cp.starts_with(&self.root))
            }
        }
    }
}
