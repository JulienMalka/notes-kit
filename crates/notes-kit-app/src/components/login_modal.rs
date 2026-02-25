use leptos::prelude::*;

use crate::hooks::use_auth;

#[component]
pub fn LoginModal() -> impl IntoView {
    let auth = use_auth();
    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(None::<String>);

    let close = move || {
        auth.close_login_prompt();
        set_email.set(String::new());
        set_password.set(String::new());
        set_error.set(None);
    };

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_error.set(None);
        let e = email.get();
        let p = password.get();
        if e.is_empty() || p.is_empty() {
            set_error.set(Some("Please fill in all fields".to_string()));
            return;
        }
        auth.do_login(e, p);
    };

    view! {
        <Show when=move || auth.login_prompt.with(|p| p.is_some())>
            <div class="onk-login-overlay" on:click=move |_| close()>
                <div class="onk-login-modal" on:click=|ev| ev.stop_propagation()>
                    <h2 class="onk-login-title">"Sign In"</h2>
                    {move || {
                        let prompt = auth.login_prompt.get().unwrap_or_default();
                        (!prompt.is_empty()).then(|| view! {
                            <p class="onk-login-prompt">{prompt}</p>
                        })
                    }}
                    {move || error.get().map(|e| view! {
                        <p class="onk-login-error">{e}</p>
                    })}
                    <form class="onk-login-form" on:submit=on_submit>
                        <input
                            class="onk-login-input"
                            type="email"
                            placeholder="Email"
                            prop:value=move || email.get()
                            on:input=move |ev| set_email.set(event_target_value(&ev))
                        />
                        <input
                            class="onk-login-input"
                            type="password"
                            placeholder="Password"
                            prop:value=move || password.get()
                            on:input=move |ev| set_password.set(event_target_value(&ev))
                        />
                        <div class="onk-login-actions">
                            <button type="submit" class="onk-login-submit">"Sign In"</button>
                            <button type="button" class="onk-login-cancel" on:click=move |_| close()>"Cancel"</button>
                        </div>
                    </form>
                </div>
            </div>
        </Show>
    }
}
