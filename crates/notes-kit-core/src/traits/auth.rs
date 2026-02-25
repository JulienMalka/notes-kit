use crate::error::AuthError;
use crate::models::{AccessGrants, Credentials, User};
use async_trait::async_trait;

#[async_trait]
pub trait AuthBackend: Send + Sync + 'static {
    async fn authenticate(&self, credentials: Credentials) -> Result<Option<User>, AuthError>;

    async fn get_user(&self, user_id: &str) -> Result<Option<User>, AuthError>;
}

pub trait AuthzPolicy: Send + Sync + 'static {
    fn effective_signature<'a>(&'a self, note_signature: Option<&'a str>) -> &'a str;

    fn anonymous_grants(&self) -> AccessGrants;

    fn can_access(&self, grants: &AccessGrants, note_signature: Option<&str>) -> bool {
        let effective = self.effective_signature(note_signature);
        grants.contains(effective)
    }

    fn grants_for_levels(&self, levels: &[String]) -> AccessGrants;
}
