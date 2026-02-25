use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct SearchShortcut {
    pub show: Signal<bool>,
    pub open: Callback<()>,
    pub close: Callback<()>,
}

pub fn use_search_shortcut() -> SearchShortcut {
    let (show, set_show) = signal(false);

    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;
        let cb = Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
            if (ev.ctrl_key() || ev.meta_key()) && ev.key() == "k" {
                ev.prevent_default();
                set_show.update(|v| *v = !*v);
            }
        }) as Box<dyn Fn(_)>);
        let window = web_sys::window().unwrap();
        let _ = window.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    SearchShortcut {
        show: Signal::from(show),
        open: Callback::new(move |_| set_show.set(true)),
        close: Callback::new(move |_| set_show.set(false)),
    }
}
