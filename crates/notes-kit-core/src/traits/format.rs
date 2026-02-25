use crate::models::{NoteId, NoteMetadata};

pub trait NoteFormat: Send + Sync + 'static {
    fn extract_metadata(&self, content: &str, filename: &str) -> NoteMetadata;

    fn parse_id(&self, filename: &str) -> Option<NoteId>;

    fn file_extension(&self) -> &str;
}
