use leptos::prelude::*;
use leptos_router::components::A;
use notes_kit_core::models::NotesConfig;

use crate::context::NotesContext;

#[component]
pub fn NotesListPage() -> impl IntoView {
    let ctx = expect_context::<NotesContext>();
    let notes_config = use_context::<NotesConfig>().unwrap_or_default();

    view! {
        <div class="onk-notes-list">
            <h1 class="onk-notes-list-title">"Notes"</h1>
            <Suspense fallback=|| view! { <p class="onk-notes-list-loading">"Loading notes..."</p> }>
                {move || {
                    let nc = notes_config.clone();
                    Suspend::new(async move {
                    match ctx.all_notes.await {
                        Ok(notes) => {
                            let notes: Vec<_> = notes.into_iter()
                                .filter(|n| !n.filename.contains("--index"))
                                .collect();
                            if notes.is_empty() {
                                view! { <p class="onk-notes-list-empty">"No notes found."</p> }.into_any()
                            } else {
                                view! {
                                    <ul class="onk-notes-list-items">
                                        {notes.into_iter().map(|note| {
                                            let path = note.path.clone();
                                            let title = note.display_title().to_string();
                                            let date = note.metadata.date.clone().unwrap_or_default();
                                            let sig = note.signature().to_string();
                                            view! {
                                                <li class="onk-notes-list-item">
                                                    <A href=nc.note_url(&path) attr:class="onk-notes-list-link">
                                                        <span class="onk-notes-list-item-title">{title}</span>
                                                        <span class="onk-notes-list-item-meta">
                                                            {if !date.is_empty() {
                                                                Some(view! { <time class="onk-notes-list-item-date">{date}</time> })
                                                            } else {
                                                                None
                                                            }}
                                                            {if sig != "public" {
                                                                Some(view! { <span class="onk-notes-list-item-sig">{sig}</span> })
                                                            } else {
                                                                None
                                                            }}
                                                        </span>
                                                    </A>
                                                </li>
                                            }
                                        }).collect_view()}
                                    </ul>
                                }.into_any()
                            }
                        }
                        Err(e) => view! { <p class="onk-notes-list-error">{format!("Error: {e}")}</p> }.into_any(),
                    }
                })}}
            </Suspense>
        </div>
    }
}
