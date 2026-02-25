use notes_kit_core::models::Note;
use std::collections::HashMap;

#[derive(Default)]
pub struct NotesCache {
    notes: Option<HashMap<String, Note>>,
}

impl NotesCache {
    pub fn get_all(&self) -> Option<Vec<Note>> {
        self.notes.as_ref().map(|m| m.values().cloned().collect())
    }

    pub fn get(&self, path: &str) -> Option<Note> {
        self.notes.as_ref()?.get(path).cloned()
    }

    pub fn set_all(&mut self, notes: Vec<Note>) {
        let mut map = HashMap::with_capacity(notes.len());
        for note in notes {
            map.insert(note.path.clone(), note);
        }
        self.notes = Some(map);
    }

    pub fn compute_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        if let Some(ref notes) = self.notes {
            let mut paths: Vec<&String> = notes.keys().collect();
            paths.sort();
            for path in paths {
                path.hash(&mut hasher);
                if let Some(ref content) = notes[path].content {
                    content.hash(&mut hasher);
                }
            }
        }
        hasher.finish()
    }
}
