use leptos::prelude::*;
use notes_kit_core::models::Note;

/// How this session should backfill its client-side corpus.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CorpusInfo {
    /// URL hash of the current public corpus snapshot
    /// (`/data/corpus/<hash>.json`), if one is built.
    pub snapshot_hash: Option<String>,
    /// Whether this session holds grants beyond anonymous. If so, its
    /// corpus contains private notes that are deliberately absent from
    /// the shared snapshot, and the backfill must use the grant-scoped
    /// (uncached) server fn instead.
    pub authed: bool,
}

#[server(GetCorpusInfo, "/api")]
pub async fn get_corpus_info() -> Result<CorpusInfo, ServerFnError> {
    let state = expect_context::<notes_kit_server::state::AppState>();
    let grants = crate::extract_grants().await?;
    let anon = state.authz_policy.anonymous_grants();

    Ok(CorpusInfo {
        snapshot_hash: state.corpus_snapshots.current_hash(),
        authed: grants.0 != anon.0,
    })
}

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

/// Summary index: every accessible note without body or highlights. List
/// pages, graphs, and search read this; full bodies arrive later via the
/// client-side corpus backfill.
#[server(GetNotesSummary, "/api")]
pub async fn get_notes_summary() -> Result<Vec<Note>, ServerFnError> {
    let state = expect_context::<notes_kit_server::state::AppState>();
    let grants = crate::extract_grants().await?;

    let mut notes = state
        .repository
        .list_accessible(&grants)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    notes.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(notes.iter().map(|n| n.summary_entry()).collect())
}

/// One note with full body and highlights — the per-route slice for note
/// pages.
#[server(GetNote, "/api")]
pub async fn get_note(path: String) -> Result<Note, ServerFnError> {
    let state = expect_context::<notes_kit_server::state::AppState>();
    let grants = crate::extract_grants().await?;

    state
        .repository
        .get_note(&path, &grants)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
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
