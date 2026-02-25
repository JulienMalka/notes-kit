use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use notes_kit_core::models::NotesConfig;

use crate::hooks::{use_search, SearchState};

#[derive(Clone, Copy)]
pub struct SearchModalState {
    pub search: SearchState,
    pub close: Callback<()>,
    pub navigate: Callback<String>,
    pub on_keydown: Callback<leptos::ev::KeyboardEvent>,
}

pub fn use_search_modal(on_close: Callback<()>) -> SearchModalState {
    let notes_config = StoredValue::new(use_context::<NotesConfig>().unwrap_or_default());
    let search = use_search();
    let nav = StoredValue::new(use_navigate());

    let close = Callback::new(move |_| {
        search.clear();
        on_close.run(());
    });

    let navigate = Callback::new(move |path: String| {
        close.run(());
        nav.with_value(|n| n(&path, Default::default()));
    });

    let on_keydown = Callback::new(move |ev: leptos::ev::KeyboardEvent| {
        match ev.key().as_str() {
            "Escape" => close.run(()),
            "ArrowDown" => {
                ev.prevent_default();
                search.select_next();
            }
            "ArrowUp" => {
                ev.prevent_default();
                search.select_prev();
            }
            "Enter" => {
                ev.prevent_default();
                if let Some(result) = search.selected_result() {
                    navigate.run(notes_config.with_value(|nc| nc.note_url(&result.path)));
                }
            }
            _ => {}
        }
    });

    SearchModalState {
        search,
        close,
        navigate,
        on_keydown,
    }
}
