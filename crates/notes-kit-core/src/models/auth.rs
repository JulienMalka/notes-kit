use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessGrants(pub HashSet<String>);

impl AccessGrants {
    pub fn new(grants: HashSet<String>) -> Self {
        Self(grants)
    }

    pub fn contains(&self, signature: &str) -> bool {
        self.0.contains(signature)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub assigned_levels: Vec<String>,
    #[serde(default)]
    pub session_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserInfo {
    pub email: String,
    pub display_name: Option<String>,
    pub assigned_levels: Vec<String>,
}

impl From<User> for UserInfo {
    fn from(u: User) -> Self {
        Self {
            email: u.email,
            display_name: u.display_name,
            assigned_levels: u.assigned_levels,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Credentials {
    Password { email: String, password: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLevelConfig {
    pub name: String,
    #[serde(default)]
    pub grants: HashSet<String>,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}
