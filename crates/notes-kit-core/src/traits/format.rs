use crate::models::{NoteId, NoteMetadata, SummaryFields};

pub trait NoteFormat: Send + Sync + 'static {
    fn extract_metadata(&self, content: &str, filename: &str) -> NoteMetadata;

    fn parse_id(&self, filename: &str) -> Option<NoteId>;

    fn file_extension(&self) -> &str;

    /// Derive body-independent summary data (excerpt, reading time, link
    /// ids, structured properties) at cache-load time. Formats that don't
    /// support this return the default (everything `None`).
    fn summary_fields(&self, _content: &str) -> SummaryFields {
        SummaryFields::default()
    }
}
