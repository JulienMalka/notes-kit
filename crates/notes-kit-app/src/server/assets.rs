use leptos::prelude::*;
use notes_kit_core::models::Asset;

#[server(GetAllAssets, "/api")]
pub async fn get_all_assets() -> Result<Vec<Asset>, ServerFnError> {
    let state = expect_context::<notes_kit_server::state::AppState>();
    let grants = crate::extract_grants().await?;

    match &state.asset_repository {
        Some(repo) => Ok(repo.list_accessible(&grants)),
        None => Ok(Vec::new()),
    }
}
