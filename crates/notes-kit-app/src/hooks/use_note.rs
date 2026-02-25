use std::collections::HashSet;

use notes_kit_core::compute::compute_id_map;
use notes_kit_core::models::{Note, NotesConfig};
use notes_kit_org::render_config::{RenderConfig, RenderContext};

use crate::context::NotesContext;

pub struct NoteData {
    pub note: Note,
    pub title: String,
    pub date: Option<String>,
    pub signature: String,
    pub content: String,
    pub denote_id: String,
    pub note_type: Option<String>,
    pub render_ctx: RenderContext,
}

pub enum NoteResult {
    Found(NoteData),
    NotFound,
    Error(String),
}

pub async fn load_note(
    ctx: NotesContext,
    path: Option<String>,
    custom_config: Option<RenderConfig>,
    notes_config: NotesConfig,
) -> NoteResult {
    match ctx.all_notes.await {
        Ok(notes) => {
            let id_map = compute_id_map(&notes);
            let accessible_ids: HashSet<String> = notes
                .iter()
                .filter_map(|n| n.metadata.id.as_ref().map(|id| id.as_str().to_string()))
                .collect();

            let mut render_ctx =
                RenderContext::new(id_map, accessible_ids).with_notes_prefix(notes_config.prefix);
            if let Some(config) = custom_config {
                render_ctx.config = config;
            }

            let found = match path {
                Some(ref p) => notes.into_iter().find(|n| n.path == *p),
                None => notes.into_iter().find(|n| n.filename.contains("--index")),
            };

            match found {
                Some(note) => {
                    let title = note.display_title().to_string();
                    let date = note.metadata.date.clone();
                    let signature = note.signature().to_string();
                    let content = note.content.clone().unwrap_or_default();
                    let denote_id = note
                        .metadata
                        .id
                        .as_ref()
                        .map(|id| id.as_str().to_string())
                        .unwrap_or_default();
                    let note_type = note.metadata.note_type.clone();

                    NoteResult::Found(NoteData {
                        note,
                        title,
                        date,
                        signature,
                        content,
                        denote_id,
                        note_type,
                        render_ctx,
                    })
                }
                None => NoteResult::NotFound,
            }
        }
        Err(e) => NoteResult::Error(format!("{e}")),
    }
}
