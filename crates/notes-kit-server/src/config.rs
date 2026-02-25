use std::path::PathBuf;

pub use notes_kit_core::models::SiteConfig;

pub struct ServerConfig {
    pub notes_dir: PathBuf,
    pub port: u16,
    pub host: String,
    pub auth_config: Option<String>,
    pub user_db_path: String,
    pub site: SiteConfig,
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
        }
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
