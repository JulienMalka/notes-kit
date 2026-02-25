use leptos::prelude::*;
use notes_kit_core::models::UserInfo;

use crate::context::NotesContext;
use crate::server::auth::{get_current_user, login, logout};

#[derive(Clone, Copy)]
pub struct AuthState {
    pub user: RwSignal<Option<UserInfo>>,
    pub login_prompt: RwSignal<Option<String>>,
}

impl AuthState {
    pub fn is_authenticated(&self) -> bool {
        self.user.with(|u| u.is_some())
    }

    pub fn request_login(&self, required_signature: &str) {
        self.login_prompt
            .set(Some(format!("This content requires '{required_signature}' access")));
    }

    pub fn request_login_any(&self) {
        self.login_prompt.set(Some(String::new()));
    }

    pub fn close_login_prompt(&self) {
        self.login_prompt.set(None);
    }

    pub fn do_login(&self, email: String, password: String) {
        let auth = *self;
        leptos::task::spawn_local(async move {
            match login(email, password).await {
                Ok(()) => {
                    if let Ok(user) = get_current_user().await {
                        auth.user.set(user);
                    }
                    auth.login_prompt.set(None);
                    if let Some(ctx) = use_context::<NotesContext>() {
                        ctx.bump_version();
                    }
                }
                Err(e) => {
                    auth.login_prompt.set(Some(format!("Login failed: {e}")));
                }
            }
        });
    }

    pub fn do_logout(&self) {
        let auth = *self;
        leptos::task::spawn_local(async move {
            let _ = logout().await;
            auth.user.set(None);
            if let Some(ctx) = use_context::<NotesContext>() {
                ctx.bump_version();
            }
        });
    }
}

pub fn use_auth() -> AuthState {
    expect_context::<AuthState>()
}

#[component]
pub fn AuthProvider(children: Children) -> impl IntoView {
    let user = RwSignal::new(None::<UserInfo>);
    let login_prompt = RwSignal::new(None::<String>);

    let auth_state = AuthState { user, login_prompt };
    provide_context(auth_state);

    let current_user = Resource::new(|| (), |_| get_current_user());

    view! {
        <Suspense fallback=|| ()>
            {move || {
                if let Some(Ok(u)) = current_user.get() {
                    user.set(u);
                }
            }}
        </Suspense>
        {children()}
        <crate::components::LoginModal />
    }
}
