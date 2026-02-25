use async_trait::async_trait;
use futures::future::join_all;
use notes_kit_core::error::RepositoryError;
use notes_kit_core::models::{AccessGrants, Note};
use notes_kit_core::traits::{AuthzPolicy, NoteFormat, NoteRepository, StorageBackend};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

use crate::cache::NotesCache;

pub struct DefaultRepository {
    storage: Arc<dyn StorageBackend>,
    format: Arc<dyn NoteFormat>,
    authz: Arc<dyn AuthzPolicy>,
    cache: Arc<RwLock<NotesCache>>,
}

impl DefaultRepository {
    pub fn new(
        storage: Arc<dyn StorageBackend>,
        format: Arc<dyn NoteFormat>,
        authz: Arc<dyn AuthzPolicy>,
        cache: Arc<RwLock<NotesCache>>,
    ) -> Self {
        Self {
            storage,
            format,
            authz,
            cache,
        }
    }

    async fn load_from_storage(&self, path: &str) -> Result<Note, RepositoryError> {
        if !self.storage.is_path_safe(path) {
            return Err(RepositoryError::NotFound("invalid path".into()));
        }
        let content = self.storage.read_file(path).await?;
        let filename = path.rsplit('/').next().unwrap_or(path);
        let metadata = self.format.extract_metadata(&content, filename);

        Ok(Note {
            path: path.to_string(),
            filename: filename.to_string(),
            content: Some(content),
            metadata,
            effective_signature: None,
        })
    }

    async fn load_all_from_storage(&self) -> Result<Vec<Note>, RepositoryError> {
        let ext = self.format.file_extension();
        let paths = self.storage.list_files(ext).await?;

        let futs: Vec<_> = paths
            .iter()
            .map(|p| {
                let p = p.clone();
                async move { self.load_from_storage(&p).await.ok() }
            })
            .collect();

        let results = join_all(futs).await;
        Ok(results.into_iter().flatten().collect())
    }

    pub async fn init_cache(&self) -> Result<(), RepositoryError> {
        let notes = self.load_all_from_storage().await?;
        self.cache.write().unwrap_or_else(|e| e.into_inner()).set_all(notes);
        Ok(())
    }

    pub async fn refresh_cache(&self) -> Result<(), RepositoryError> {
        let notes = self.load_all_from_storage().await?;
        self.cache.write().unwrap_or_else(|e| e.into_inner()).set_all(notes);
        Ok(())
    }

    pub fn global_version_hash(&self) -> u64 {
        self.cache.read().unwrap_or_else(|e| e.into_inner()).compute_hash()
    }

    fn apply_effective_signature(&self, mut note: Note) -> Note {
        let effective = self
            .authz
            .effective_signature(note.metadata.signature.as_deref())
            .to_string();
        note.effective_signature = Some(effective);
        note
    }
}

#[async_trait]
impl NoteRepository for DefaultRepository {
    async fn list_accessible(&self, grants: &AccessGrants) -> Result<Vec<Note>, RepositoryError> {
        let all = self.get_all().await?;
        Ok(all
            .into_iter()
            .filter(|n| self.authz.can_access(grants, n.metadata.signature.as_deref()))
            .map(|n| self.apply_effective_signature(n))
            .collect())
    }

    async fn get_note(
        &self,
        path: &str,
        grants: &AccessGrants,
    ) -> Result<Note, RepositoryError> {
        let note = self.get_unchecked(path).await?;
        if self
            .authz
            .can_access(grants, note.metadata.signature.as_deref())
        {
            Ok(self.apply_effective_signature(note))
        } else {
            let sig = self
                .authz
                .effective_signature(note.metadata.signature.as_deref());
            Err(RepositoryError::Unauthorized(format!(
                "Authentication required for '{sig}' access"
            )))
        }
    }

    async fn get_all(&self) -> Result<Vec<Note>, RepositoryError> {
        if let Some(notes) = self.cache.read().unwrap_or_else(|e| e.into_inner()).get_all() {
            return Ok(notes);
        }
        self.load_all_from_storage().await
    }

    async fn get_unchecked(&self, path: &str) -> Result<Note, RepositoryError> {
        if let Some(note) = self.cache.read().unwrap_or_else(|e| e.into_inner()).get(path) {
            return Ok(note);
        }
        self.load_from_storage(path).await
    }

    async fn version_hash(&self, grants: &AccessGrants) -> Result<u64, RepositoryError> {
        let notes = self.list_accessible(grants).await?;
        let mut hasher = DefaultHasher::new();
        for note in &notes {
            note.path.hash(&mut hasher);
            if let Some(ref content) = note.content {
                content.hash(&mut hasher);
            }
        }
        Ok(hasher.finish())
    }
}
