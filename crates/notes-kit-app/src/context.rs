use leptos::prelude::*;
use notes_kit_core::models::Note;

use crate::server::notes::get_all_notes;

#[derive(Clone, Copy)]
pub struct NotesContext {
    pub version: RwSignal<u64>,
    pub all_notes: Resource<Result<Vec<Note>, ServerFnError>>,
}

impl NotesContext {
    pub fn bump_version(&self) {
        self.version.update(|v| *v = v.wrapping_add(1));
    }
}

#[component]
pub fn QueryProvider(children: Children) -> impl IntoView {
    let version = RwSignal::new(0u64);

    let all_notes = Resource::new(
        move || version.get(),
        |_| get_all_notes(),
    );

    let ctx = NotesContext {
        version,
        all_notes,
    };

    provide_context(ctx);

    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;
        let es = web_sys::EventSource::new("/api/events/notes").unwrap();
        let cb = Closure::wrap(Box::new(move |_: web_sys::MessageEvent| {
            ctx.bump_version();
        }) as Box<dyn Fn(web_sys::MessageEvent)>);
        es.set_onmessage(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
        std::mem::forget(es);
    }

    children()
}
