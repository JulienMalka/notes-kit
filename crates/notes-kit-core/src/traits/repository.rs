use crate::error::RepositoryError;
use crate::models::{AccessGrants, Note};
use async_trait::async_trait;

#[async_trait]
pub trait NoteRepository: Send + Sync + 'static {
    async fn list_accessible(&self, grants: &AccessGrants) -> Result<Vec<Note>, RepositoryError>;

    async fn get_note(
        &self,
        path: &str,
        grants: &AccessGrants,
    ) -> Result<Note, RepositoryError>;

    async fn get_all(&self) -> Result<Vec<Note>, RepositoryError>;

    async fn get_unchecked(&self, path: &str) -> Result<Note, RepositoryError>;

    async fn version_hash(&self, grants: &AccessGrants) -> Result<u64, RepositoryError>;
}
