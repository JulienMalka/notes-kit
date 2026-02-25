use crate::models::NoteId;
use std::collections::HashSet;

pub fn extract_denote_link_ids(content: &str) -> HashSet<NoteId> {
    let mut ids = HashSet::new();
    let pattern = "[[denote:";

    for (idx, _) in content.match_indices(pattern) {
        let start = idx + pattern.len();
        if start + 15 <= content.len() {
            let potential = &content[start..start + 15];
            if potential.as_bytes()[8] == b'T'
                && potential.bytes().enumerate().all(|(i, b)| i == 8 || b.is_ascii_digit())
            {
                ids.insert(NoteId::new(potential));
            }
        }
    }

    ids
}
