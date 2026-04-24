use notes_kit_core::error::{RepositoryError, StorageError};
use notes_kit_core::models::{AccessGrants, Asset};
use notes_kit_core::traits::{AuthzPolicy, StorageBackend};
use notes_kit_org::denote::DenoteFilename;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

pub struct AssetRepository {
    storage: Arc<dyn StorageBackend>,
    authz: Arc<dyn AuthzPolicy>,
    cache: RwLock<Vec<Asset>>,
}

impl AssetRepository {
    pub fn new(storage: Arc<dyn StorageBackend>, authz: Arc<dyn AuthzPolicy>) -> Self {
        Self {
            storage,
            authz,
            cache: RwLock::new(Vec::new()),
        }
    }

    fn parse_asset(path: &str) -> Option<Asset> {
        let filename = path.rsplit('/').next().unwrap_or(path);
        let parsed = DenoteFilename::parse(filename)?;
        if parsed.is_note() {
            return None;
        }
        Some(Asset {
            path: path.to_string(),
            filename: filename.to_string(),
            denote_id: parsed.id.as_str().to_string(),
            signature: parsed.signature,
        })
    }

    pub async fn init_cache(&self) -> Result<(), RepositoryError> {
        let files = self.storage.list_all_files().await?;
        let assets: Vec<Asset> = files.iter().filter_map(|p| Self::parse_asset(p)).collect();
        *self.cache.write().unwrap_or_else(|e| e.into_inner()) = assets;
        Ok(())
    }

    pub async fn refresh_cache(&self) -> Result<(), RepositoryError> {
        self.init_cache().await
    }

    pub async fn listing_hash(&self) -> Result<u64, RepositoryError> {
        let mut files = self.storage.list_all_files().await?;
        files.sort();
        let mut hasher = DefaultHasher::new();
        for p in &files {
            p.hash(&mut hasher);
        }
        Ok(hasher.finish())
    }

    pub fn cached_asset_count(&self) -> usize {
        self.cache.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn list_accessible(&self, grants: &AccessGrants) -> Vec<Asset> {
        let cache = self.cache.read().unwrap_or_else(|e| e.into_inner());
        cache
            .iter()
            .filter(|a| self.authz.can_access(grants, a.signature.as_deref()))
            .cloned()
            .collect()
    }

    pub fn can_access_asset(&self, signature: Option<&str>, grants: &AccessGrants) -> bool {
        self.authz.can_access(grants, signature)
    }

    pub async fn read_bytes(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        if !self.storage.is_path_safe(path) {
            return Err(StorageError::InvalidPath(format!("unsafe path: {path}")));
        }
        self.storage.read_file_bytes(path).await
    }
}
