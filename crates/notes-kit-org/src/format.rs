use notes_kit_core::models::{NoteId, NoteMetadata, SummaryFields};
use notes_kit_core::traits::NoteFormat;

use crate::denote::DenoteFilename;
use crate::helpers::extract_metadata_fast;
use crate::text::{extract_excerpt, extract_section, parse_properties, reading_time};

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

    fn summary_fields(&self, content: &str) -> SummaryFields {
        let mut properties: std::collections::BTreeMap<String, String> =
            parse_properties(content)
                .into_iter()
                .map(|(k, v)| (k.to_ascii_uppercase(), v))
                .collect();
        // The Abstract section doubles as a structured field (research
        // cards render it); an explicit :ABSTRACT: property wins.
        if let Some(abstract_text) = extract_section(content, "Abstract") {
            properties
                .entry("ABSTRACT".to_string())
                .or_insert(abstract_text);
        }

        // Sorted so the serialized form is deterministic (the corpus
        // snapshot URL is a hash of the serialized bytes).
        let mut link_ids: Vec<NoteId> =
            notes_kit_core::compute::extract_denote_link_ids(content)
                .into_iter()
                .collect();
        link_ids.sort();

        SummaryFields {
            excerpt: Some(extract_excerpt(content, 250)),
            reading_minutes: Some(reading_time(content).min(u16::MAX as usize) as u16),
            link_ids: (!link_ids.is_empty()).then_some(link_ids),
            properties: (!properties.is_empty()).then_some(properties),
        }
    }
}
