use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub title: String,
    pub site_url: Option<String>,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            title: "My Notes".to_string(),
            site_url: None,
            author_name: None,
            author_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotesConfig {
    pub prefix: String,
}

impl Default for NotesConfig {
    fn default() -> Self {
        Self {
            prefix: "/notes".into(),
        }
    }
}

impl NotesConfig {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    pub fn note_url(&self, file_path: &str) -> String {
        format!("{}/{}", self.prefix, file_path)
    }
}
