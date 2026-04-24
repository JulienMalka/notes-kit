use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub path: String,
    pub filename: String,
    pub denote_id: String,
    pub signature: Option<String>,
}

pub type AssetMap = HashMap<String, String>;

pub fn compute_asset_map(assets: &[Asset]) -> AssetMap {
    assets
        .iter()
        .map(|a| (a.denote_id.clone(), a.filename.clone()))
        .collect()
}
