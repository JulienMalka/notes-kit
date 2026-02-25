use std::sync::Arc;

use axum_login::{AuthUser, AuthnBackend, UserId};
use notes_kit_core::models::{Credentials, User};
use notes_kit_core::traits::AuthBackend;

#[derive(Clone, Debug)]
pub struct SessionUser(pub User);

impl AuthUser for SessionUser {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.0.id.clone()
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.0.session_hash.as_bytes()
    }
}

#[derive(Clone)]
pub struct DynAuthnBackend(pub Arc<dyn AuthBackend>);

impl AuthnBackend for DynAuthnBackend {
    type User = SessionUser;
    type Credentials = Credentials;
    type Error = std::convert::Infallible;

    fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> impl std::future::Future<Output = Result<Option<Self::User>, Self::Error>> + Send {
        let backend = self.0.clone();
        async move {
            Ok(backend
                .authenticate(credentials)
                .await
                .ok()
                .flatten()
                .map(SessionUser))
        }
    }

    fn get_user(
        &self,
        user_id: &UserId<Self>,
    ) -> impl std::future::Future<Output = Result<Option<Self::User>, Self::Error>> + Send {
        let backend = self.0.clone();
        let uid = user_id.to_string();
        async move {
            Ok(backend
                .get_user(&uid)
                .await
                .ok()
                .flatten()
                .map(SessionUser))
        }
    }
}

pub type AuthSession = axum_login::AuthSession<DynAuthnBackend>;
