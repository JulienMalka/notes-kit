use leptos::prelude::*;
use notes_kit_core::models::{Asset, Note};

use crate::server::assets::get_all_assets;
use crate::server::notes::get_all_notes;

#[derive(Clone, Copy)]
pub struct NotesContext {
    pub version: RwSignal<u64>,
    pub all_notes: Resource<Result<Vec<Note>, ServerFnError>>,
    pub all_assets: Resource<Result<Vec<Asset>, ServerFnError>>,
}

impl NotesContext {
    pub fn bump_version(&self) {
        self.version.update(|v| *v = v.wrapping_add(1));
    }
}

#[component]
pub fn QueryProvider(children: Children) -> impl IntoView {
    let version = RwSignal::new(0u64);

    let all_notes = Resource::new(move || version.get(), |_| get_all_notes());

    let all_assets = Resource::new(move || version.get(), |_| get_all_assets());

    let ctx = NotesContext {
        version,
        all_notes,
        all_assets,
    };

    provide_context(ctx);

    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;
        let window = web_sys::window().unwrap();
        let cb = Closure::once(Box::new(move || {
            let es = web_sys::EventSource::new("/api/events/notes").unwrap();
            let on_msg = Closure::wrap(Box::new(move |_: web_sys::MessageEvent| {
                ctx.bump_version();
            }) as Box<dyn Fn(web_sys::MessageEvent)>);
            es.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
            on_msg.forget();
            let on_err = Closure::wrap(Box::new(move |_: web_sys::Event| {
                web_sys::console::warn_1(&"[sse] connection lost, reconnecting...".into());
            }) as Box<dyn Fn(web_sys::Event)>);
            es.set_onerror(Some(on_err.as_ref().unchecked_ref()));
            on_err.forget();
            std::mem::forget(es);
        }) as Box<dyn FnOnce()>);

        // requestIdleCallback when available
        let opts = web_sys::IdleRequestOptions::new();
        opts.set_timeout(3000);
        let scheduled =
            window.request_idle_callback_with_options(cb.as_ref().unchecked_ref(), &opts);
        if scheduled.is_err() {
            let already_loaded = window
                .document()
                .map(|d| d.ready_state() == "complete")
                .unwrap_or(false);
            if already_loaded {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    0,
                );
            } else {
                let _ =
                    window.add_event_listener_with_callback("load", cb.as_ref().unchecked_ref());
            }
        }
        cb.forget();
    }

    children()
}
