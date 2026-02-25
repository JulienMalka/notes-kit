use leptos::prelude::*;
use notes_kit_core::models::NotesConfig;

use crate::hooks::use_auth;

#[component]
pub fn Header(
    #[prop(optional, into)]
    title: Option<String>,
    #[prop(into)]
    on_search: Callback<()>,
) -> impl IntoView {
    let title = title.unwrap_or_else(|| {
        use_context::<notes_kit_core::models::SiteConfig>()
            .map(|c| c.title.clone())
            .unwrap_or_else(|| "My Notes".to_string())
    });

    let notes_config = use_context::<NotesConfig>().unwrap_or_default();
    let auth = use_auth();

    view! {
        <header class="onk-header">
            <div class="onk-header-content">
                <a href="/" class="onk-header-brand">{title}</a>
                <nav class="onk-header-nav">
                    <button class="onk-header-search-btn" on:click=move |_| on_search.run(())>
                        "Search"
                    </button>
                    <a href=notes_config.prefix.clone() class="onk-header-nav-link">"Notes"</a>
                    <Show
                        when=move || !auth.is_authenticated()
                        fallback=|| view! {}
                    >
                        <button
                            class="onk-header-login-btn"
                            on:click=move |_| auth.request_login_any()
                        >
                            "Login"
                        </button>
                    </Show>
                </nav>
            </div>
        </header>
    }
}
