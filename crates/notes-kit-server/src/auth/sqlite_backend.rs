use async_trait::async_trait;
use notes_kit_core::error::AuthError;
use notes_kit_core::models::{Credentials, User};
use notes_kit_core::traits::AuthBackend;

use super::user_repository::UserRepository;

pub struct SqliteAuthBackend {
    repo: UserRepository,
}

impl SqliteAuthBackend {
    pub fn new(repo: UserRepository) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl AuthBackend for SqliteAuthBackend {
    async fn authenticate(&self, credentials: Credentials) -> Result<Option<User>, AuthError> {
        match credentials {
            Credentials::Password { email, password } => {
                let result = self
                    .repo
                    .verify_password(&email, &password)
                    .await
                    .map_err(|e| AuthError::Internal(e.to_string()))?;
                Ok(result.map(|su| su.to_core_user()))
            }
        }
    }

    async fn get_user(&self, user_id: &str) -> Result<Option<User>, AuthError> {
        let result = self
            .repo
            .get_user(user_id)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        Ok(result.map(|su| su.to_core_user()))
    }
}
