use notes_kit_core::models::{AccessGrants, AccessLevelConfig};
use notes_kit_core::traits::AuthzPolicy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Deserialize)]
pub struct AnonymousConfig {
    #[serde(default = "default_anonymous_grants")]
    pub grants: HashSet<String>,
}

impl Default for AnonymousConfig {
    fn default() -> Self {
        Self {
            grants: default_anonymous_grants(),
        }
    }
}

fn default_anonymous_grants() -> HashSet<String> {
    ["public".to_string()].into_iter().collect()
}

fn default_signature() -> String {
    "private".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminUserConfig {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub assigned_levels: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfigFile {
    #[serde(default = "default_signature")]
    pub default_signature: String,
    #[serde(default)]
    pub anonymous: AnonymousConfig,
    #[serde(default)]
    pub levels: Vec<AccessLevelConfig>,
    pub admin: Option<AdminUserConfig>,
}

impl Default for AuthConfigFile {
    fn default() -> Self {
        Self {
            default_signature: default_signature(),
            anonymous: AnonymousConfig::default(),
            levels: Vec::new(),
            admin: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigAuthzPolicy {
    levels: HashMap<String, AccessLevelConfig>,
    default_signature: String,
    anonymous_grants: HashSet<String>,
}

impl ConfigAuthzPolicy {
    pub fn from_config(config: AuthConfigFile) -> Self {
        let mut levels = HashMap::new();
        for mut level in config.levels {
            level.grants.insert(level.name.clone());
            levels.insert(level.name.clone(), level);
        }
        Self {
            levels,
            default_signature: config.default_signature,
            anonymous_grants: config.anonymous.grants,
        }
    }
}

impl AuthzPolicy for ConfigAuthzPolicy {
    fn effective_signature<'a>(&'a self, note_signature: Option<&'a str>) -> &'a str {
        note_signature.unwrap_or(&self.default_signature)
    }

    fn anonymous_grants(&self) -> AccessGrants {
        AccessGrants::new(self.anonymous_grants.clone())
    }

    fn grants_for_levels(&self, levels: &[String]) -> AccessGrants {
        let mut grants = HashSet::new();
        for level_name in levels {
            if let Some(level) = self.levels.get(level_name) {
                grants.extend(level.grants.iter().cloned());
            }
        }
        AccessGrants::new(grants)
    }
}
