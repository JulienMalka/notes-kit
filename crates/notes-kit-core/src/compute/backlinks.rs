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
            let content = note.content.as_deref().unwrap_or("");
            let linked_ids = extract_denote_link_ids(content);
            linked_ids.iter().any(|id| id.as_str() == target_id)
        })
        .map(|note| Note::list_entry(note.path.clone(), note.filename.clone(), note.metadata.clone()))
        .collect();

    backlinks.sort_by(|a, b| b.filename.cmp(&a.filename));
    backlinks
}
