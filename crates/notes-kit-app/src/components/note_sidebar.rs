use leptos::prelude::*;
use leptos_router::components::A;
use notes_kit_core::models::NotesConfig;

use crate::hooks::use_backlinks;

#[component]
pub fn NoteBacklinksSection(
    #[prop(into)] denote_id: String,
) -> impl IntoView {
    let notes_config = use_context::<NotesConfig>().unwrap_or_default();
    let id = denote_id.clone();
    let backlinks = use_backlinks(move || id.clone());

    view! {
        <section class="onk-note-backlinks">
            {move || {
                let nc = notes_config.clone();
                let links = backlinks.get();
                if links.is_empty() {
                    None
                } else {
                    Some(view! {
                        <h2 class="onk-note-backlinks-title">"Backlinks"</h2>
                        <ul class="onk-note-backlinks-list">
                            {links.into_iter().map(|note| {
                                let path = note.path.clone();
                                let title = note.display_title().to_string();
                                view! {
                                    <li class="onk-note-backlinks-item">
                                        <A href=nc.note_url(&path)>{title}</A>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    })
                }
            }}
        </section>
    }
}
