use std::path::PathBuf;

pub use notes_kit_core::models::SiteConfig;

pub struct ServerConfig {
    pub notes_dir: PathBuf,
    pub port: u16,
    pub host: String,
    pub auth_config: Option<String>,
    pub user_db_path: String,
    pub site: SiteConfig,
    /// Absolute base URL (e.g. "https://example.com") used to serve /sitemap.xml.
    /// If None, /sitemap.xml is not served.
    pub sitemap_base_url: Option<String>,
    /// Static paths included in the sitemap in addition to public notes.
    /// Defaults to just "/". Paths should start with "/".
    pub sitemap_static_paths: Vec<String>,
    /// Atom feed configuration; `None` disables /feed.xml.
    pub feed: Option<crate::feed::FeedConfig>,
    /// Extra application routes merged into the server's router — lets
    /// the embedding site add endpoints (e.g. generated OG images)
    /// without forking the serve loop.
    pub extra_router: Option<axum::Router>,
    /// Applied to every note as it loads from storage (initial load and
    /// every refresh) — e.g. attaching precomputed syntax highlighting.
    pub note_transform: Option<crate::repository::NoteTransform>,
}

impl ServerConfig {
    pub fn new(notes_dir: impl Into<PathBuf>) -> Self {
        Self {
            notes_dir: notes_dir.into(),
            port: 3000,
            host: "127.0.0.1".to_string(),
            auth_config: None,
            user_db_path: ".users.db".to_string(),
            site: SiteConfig::default(),
            sitemap_base_url: None,
            feed: None,
            sitemap_static_paths: vec!["/".to_string()],
            extra_router: None,
            note_transform: None,
        }
    }

    pub fn note_transform(mut self, transform: crate::repository::NoteTransform) -> Self {
        self.note_transform = Some(transform);
        self
    }

    pub fn extra_router(mut self, router: axum::Router) -> Self {
        self.extra_router = Some(router);
        self
    }

    pub fn sitemap_base_url(mut self, url: impl Into<String>) -> Self {
        self.sitemap_base_url = Some(url.into());
        self
    }

    pub fn feed(mut self, cfg: crate::feed::FeedConfig) -> Self {
        self.feed = Some(cfg);
        self
    }

    pub fn sitemap_static_paths(mut self, paths: Vec<String>) -> Self {
        self.sitemap_static_paths = paths;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.site.title = title.into();
        self
    }

    pub fn auth_config(mut self, path: impl Into<String>) -> Self {
        self.auth_config = Some(path.into());
        self
    }
}
