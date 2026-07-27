use crate::models::Note;

use super::extract_denote_link_ids;

pub fn compute_backlinks(notes: &[Note], target_id: &str) -> Vec<Note> {
    if target_id.is_empty() {
        return Vec::new();
    }

    let mut backlinks: Vec<Note> = notes
        .iter()
        .filter(|note| {
            if note.filename.starts_with(target_id) {
                return false;
            }
            if note.filename == "index.org" {
                return false;
            }
            match &note.link_ids {
                // Precomputed at cache load — works on body-less summaries.
                Some(ids) => ids.iter().any(|id| id.as_str() == target_id),
                None => {
                    let content = note.content.as_deref().unwrap_or("");
                    extract_denote_link_ids(content)
                        .iter()
                        .any(|id| id.as_str() == target_id)
                }
            }
        })
        .map(|note| note.summary_entry())
        .collect();

    backlinks.sort_by(|a, b| b.filename.cmp(&a.filename));
    backlinks
}
