use leptos::prelude::*;
use notes_kit_core::models::SearchResult;

use crate::context::NotesContext;

#[derive(Clone, Copy)]
pub struct SearchState {
    pub query: ReadSignal<String>,
    pub set_query: WriteSignal<String>,
    pub results: Memo<Vec<SearchResult>>,
    pub selected_index: RwSignal<usize>,
}

impl SearchState {
    pub fn select_next(&self) {
        let max = self.results.with(|r| r.len().saturating_sub(1));
        self.selected_index.update(|i| {
            if *i < max {
                *i += 1;
            }
        });
    }

    pub fn select_prev(&self) {
        self.selected_index.update(|i| {
            *i = i.saturating_sub(1);
        });
    }

    pub fn selected_result(&self) -> Option<SearchResult> {
        let idx = self.selected_index.get();
        self.results.with(|r| r.get(idx).cloned())
    }

    pub fn clear(&self) {
        self.set_query.set(String::new());
        self.selected_index.set(0);
    }

    pub fn has_results(&self) -> bool {
        self.results.with(|r| !r.is_empty())
    }
}

pub fn use_search() -> SearchState {
    let ctx = expect_context::<NotesContext>();
    let (query, set_query) = signal(String::new());
    let selected_index = RwSignal::new(0usize);

    let results = Memo::new(move |_| {
        let q = query.get();
        if q.len() < 2 {
            return vec![];
        }
        let notes = ctx.all_notes.get().and_then(|r| r.ok()).unwrap_or_default();
        notes_kit_core::search::search_notes(&notes, &q)
    });

    Effect::new(move |_| {
        let _ = query.get();
        selected_index.set(0);
    });

    SearchState {
        query,
        set_query,
        results,
        selected_index,
    }
}
