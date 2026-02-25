use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use notes_kit_core::models::NotesConfig;

use crate::context::NotesContext;
use crate::hooks::{load_note, NoteResult};

#[component]
pub fn NotePage() -> impl IntoView {
    let params = use_params_map();
    let note_path = move || params.with(|p| p.get("path"));

    let ctx = expect_context::<NotesContext>();
    let notes_config = use_context::<NotesConfig>().unwrap_or_default();
    let custom_render_config = use_context::<notes_kit_org::render_config::RenderConfig>();

    view! {
        <div class="onk-note-page">
            <Suspense fallback=|| view! { <div class="onk-note-loading">"Loading..."</div> }>
                {move || {
                    let path = note_path();
                    let render_config = custom_render_config.clone();
                    let nc = notes_config.clone();
                    Suspend::new(async move {
                        match load_note(ctx, path, render_config, nc).await {
                            NoteResult::Found(data) => {
                                let sig = data.signature.clone();
                                provide_context(data.render_ctx);
                                view! {
                                    <article class="onk-note">
                                        <header class="onk-note-header">
                                            <h1 class="onk-note-title">{data.title}</h1>
                                            {data.date.map(|d| view! {
                                                <time class="onk-note-date">{d}</time>
                                            })}
                                            {(sig != "public").then(|| view! {
                                                <span class="onk-note-signature">{sig}</span>
                                            })}
                                        </header>
                                        <div class="onk-note-content">
                                            <notes_kit_org::OrgContent content=data.content />
                                        </div>
                                        <crate::components::NoteBacklinksSection denote_id=data.denote_id />
                                    </article>
                                }.into_any()
                            }
                            NoteResult::NotFound => {
                                view! {
                                    <div class="onk-note-error">
                                        <p>"Note not found."</p>
                                    </div>
                                }.into_any()
                            }
                            NoteResult::Error(e) => {
                                view! {
                                    <div class="onk-note-error">
                                        <p>{format!("Error: {e}")}</p>
                                    </div>
                                }.into_any()
                            }
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
