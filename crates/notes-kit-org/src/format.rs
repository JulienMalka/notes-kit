use notes_kit_core::models::{NoteId, NoteMetadata};
use notes_kit_core::traits::NoteFormat;

use crate::denote::DenoteFilename;
use crate::helpers::extract_metadata_fast;

#[derive(Default)]
pub struct OrgFormat;

impl NoteFormat for OrgFormat {
    fn extract_metadata(&self, content: &str, filename: &str) -> NoteMetadata {
        let (title, date, tags) = extract_metadata_fast(content);
        let denote = DenoteFilename::parse(filename);

        let id = denote.as_ref().map(|d| NoteId::new(d.id.as_str()));
        let note_type = denote.as_ref().and_then(|d| d.note_type.clone());
        let signature = denote.as_ref().and_then(|d| d.signature.clone());

        let effective_date = date.or_else(|| denote.as_ref().map(|d| d.id.date()));

        NoteMetadata {
            id,
            title,
            date: effective_date,
            tags,
            note_type,
            signature,
        }
    }

    fn parse_id(&self, filename: &str) -> Option<NoteId> {
        DenoteFilename::parse(filename).map(|d| NoteId::new(d.id.as_str()))
    }

    fn file_extension(&self) -> &str {
        "org"
    }
}
