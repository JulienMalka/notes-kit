use leptos::prelude::*;
use notes_kit_core::models::Note;

#[server(GetAllNotes, "/api")]
pub async fn get_all_notes() -> Result<Vec<Note>, ServerFnError> {
    let state = expect_context::<notes_kit_server::state::AppState>();
    let grants = crate::extract_grants().await?;

    let mut notes = state
        .repository
        .list_accessible(&grants)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    notes.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(notes)
}

#[server(GetNotesVersion, "/api")]
pub async fn get_notes_version() -> Result<u64, ServerFnError> {
    let state = expect_context::<notes_kit_server::state::AppState>();
    let grants = crate::extract_grants().await?;

    state
        .repository
        .version_hash(&grants)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
