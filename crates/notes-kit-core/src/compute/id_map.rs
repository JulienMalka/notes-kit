use crate::models::{Note, NoteIdMap};

pub fn compute_id_map(notes: &[Note]) -> NoteIdMap {
    notes
        .iter()
        .filter_map(|n| {
            n.metadata
                .id
                .as_ref()
                .map(|id| (id.as_str().to_string(), n.path.clone()))
        })
        .collect()
}
