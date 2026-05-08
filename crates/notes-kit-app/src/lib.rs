#![recursion_limit = "512"]

pub mod components;
pub mod context;
pub mod hooks;
pub mod server;

pub use components::*;
pub use context::QueryProvider;
pub use hooks::*;

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

#[cfg(feature = "ssr")]
pub async fn extract_grants() -> Result<notes_kit_core::models::AccessGrants, ServerFnError> {
    use notes_kit_server::auth::AuthSession;

    let state = expect_context::<notes_kit_server::state::AppState>();
    let auth_session: AuthSession = leptos_axum::extract().await?;

    let grants = if let Some(ref user) = auth_session.user {
        state
            .authz_policy
            .grants_for_levels(&user.0.assigned_levels)
    } else {
        state.authz_policy.anonymous_grants()
    };

    Ok(grants)
}

#[component]
pub fn NotesProvider(children: Children) -> impl IntoView {
    view! {
        <QueryProvider>
            <crate::hooks::AuthProvider>
                {children()}
            </crate::hooks::AuthProvider>
        </QueryProvider>
    }
}

#[component]
pub fn DefaultApp() -> impl IntoView {
    provide_meta_context();

    let search = use_search_shortcut();

    view! {
        <NotesProvider>
            <Router>
                <components::Header on_search=search.open />
                <components::SearchModal show=search.show on_close=search.close />
                <components::AuthIndicator />
                <main class="onk-main">
                    <Routes fallback=|| "Page not found.">
                        <Route path=path!("/") view=components::NotePage />
                        <Route path=path!("/notes") view=components::NotesListPage />
                        <Route path=path!("/notes/*path") view=components::NotePage />
                    </Routes>
                </main>
            </Router>
        </NotesProvider>
    }
}

pub fn default_shell(options: leptos::prelude::LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
                <MetaTags />
                <link rel="stylesheet" href="/notes-kit.css" />
            </head>
            <body>
                <DefaultApp />
            </body>
        </html>
    }
}
