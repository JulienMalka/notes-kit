use leptos::prelude::*;
use notes_kit_core::models::Note;

use crate::context::NotesContext;

pub fn use_backlinks(denote_id: impl Fn() -> String + Send + Sync + 'static) -> Memo<Vec<Note>> {
    let ctx = expect_context::<NotesContext>();

    Memo::new(move |_| {
        let id = denote_id();
        if id.is_empty() {
            return vec![];
        }
        let notes = ctx.all_notes.get().and_then(|r| r.ok()).unwrap_or_default();
        notes_kit_core::compute::compute_backlinks(&notes, &id)
    })
}
