use notes_kit_core::traits::{AuthBackend, AuthzPolicy, NoteRepository};
use std::sync::Arc;

use crate::asset_repository::AssetRepository;
use crate::config::SiteConfig;

#[derive(Clone)]
pub struct AppState {
    pub repository: Arc<dyn NoteRepository>,
    pub auth_backend: Arc<dyn AuthBackend>,
    pub authz_policy: Arc<dyn AuthzPolicy>,
    pub site_config: SiteConfig,
    pub asset_repository: Option<Arc<AssetRepository>>,
}
