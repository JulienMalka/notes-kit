use leptos::prelude::*;

use crate::hooks::use_auth;

#[component]
pub fn AuthIndicator() -> impl IntoView {
    let auth = use_auth();

    view! {
        <Show when=move || auth.is_authenticated()>
            <div class="onk-auth-indicator">
                <span class="onk-auth-indicator-email">
                    {move || auth.user.with(|u| u.as_ref().map(|u| u.email.clone()).unwrap_or_default())}
                </span>
                <button
                    class="onk-auth-indicator-logout"
                    on:click=move |_| auth.do_logout()
                >
                    "Logout"
                </button>
            </div>
        </Show>
    }
}
