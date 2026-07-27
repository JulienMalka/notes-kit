use serde::{Deserialize, Serialize};
use std::cmp::min;

use super::NoteId;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NoteMetadata {
    pub id: Option<NoteId>,
    pub title: Option<String>,
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub note_type: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub path: String,
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub metadata: NoteMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_signature: Option<String>,
    /// Precomputed HTML fragments for code blocks, keyed by a stable hash
    /// of (language, content). Filled server-side at cache load (e.g.
    /// syntax highlighting) and shipped with the note so server and
    /// client render identical output. Ordered map so the serialized form
    /// is deterministic (the corpus snapshot URL hashes the bytes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlights: Option<std::collections::BTreeMap<u64, String>>,
    /// Plain-text prose excerpt, precomputed at cache load so list pages,
    /// cards, and hover previews render without the body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    /// Estimated reading time in minutes, computed from the full body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading_minutes: Option<u16>,
    /// Ids of the notes this note links to, extracted from the body at
    /// cache load. Lets backlinks and link graphs be computed without any
    /// note content. Sorted, so serialization is deterministic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_ids: Option<Vec<NoteId>>,
    /// Structured fields parsed from the body's top property drawer (plus
    /// format-specific extras such as an ABSTRACT section), for pages that
    /// render fields rather than prose. Keys are uppercased.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<std::collections::BTreeMap<String, String>>,
}

/// Body-derived summary data a [`crate::traits::NoteFormat`] can attach to
/// notes at load time. Everything here must be renderable without the body.
#[derive(Debug, Clone, Default)]
pub struct SummaryFields {
    pub excerpt: Option<String>,
    pub reading_minutes: Option<u16>,
    pub link_ids: Option<Vec<NoteId>>,
    pub properties: Option<std::collections::BTreeMap<String, String>>,
}

impl Note {
    pub fn list_entry(path: String, filename: String, metadata: NoteMetadata) -> Self {
        Self {
            path,
            filename,
            content: None,
            metadata,
            effective_signature: None,
            highlights: None,
            excerpt: None,
            reading_minutes: None,
            link_ids: None,
            properties: None,
        }
    }

    /// Copy of the note without body or highlights — the shape shipped in
    /// the summary index and in backlink lists.
    pub fn summary_entry(&self) -> Self {
        Self {
            path: self.path.clone(),
            filename: self.filename.clone(),
            content: None,
            metadata: self.metadata.clone(),
            effective_signature: self.effective_signature.clone(),
            highlights: None,
            excerpt: self.excerpt.clone(),
            reading_minutes: self.reading_minutes,
            link_ids: self.link_ids.clone(),
            properties: self.properties.clone(),
        }
    }

    pub fn display_title(&self) -> &str {
        self.metadata
            .title
            .as_deref()
            .unwrap_or_else(|| self.filename.strip_suffix(".org").unwrap_or(&self.filename))
    }

    pub fn signature(&self) -> &str {
        self.effective_signature
            .as_deref()
            .or(self.metadata.signature.as_deref())
            .unwrap_or("public")
    }

    pub fn content_contains_lowercase(&self, pattern_lower: &str) -> bool {
        self.content
            .as_ref()
            .is_some_and(|c| c.to_lowercase().contains(pattern_lower))
    }

    pub fn snippet_around(&self, query: &str) -> String {
        let Some(content) = &self.content else {
            return String::from("No content available");
        };

        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();

        let char_offsets: Vec<usize> = content.char_indices().map(|(i, _)| i).collect();

        if let Some(char_pos) = content_lower.find(&query_lower).map(|byte_pos| {
            content_lower[..byte_pos].chars().count()
        }) {
            let total_chars = char_offsets.len();
            let query_chars = query.chars().count();

            let start_char = char_pos.saturating_sub(40);
            let end_char = min(char_pos + query_chars + 40, total_chars);

            let start_byte = char_offsets.get(start_char).copied().unwrap_or(0);
            let pos_byte = char_offsets.get(char_pos).copied().unwrap_or(0);
            let end_byte = char_offsets.get(end_char).copied().unwrap_or(content.len());

            let snippet_start = content[start_byte..pos_byte]
                .rfind(char::is_whitespace)
                .map(|i| start_byte + i + 1)
                .unwrap_or(start_byte);
            let snippet_start = if content.is_char_boundary(snippet_start) {
                snippet_start
            } else {
                (snippet_start..content.len())
                    .find(|&i| content.is_char_boundary(i))
                    .unwrap_or(content.len())
            };

            let snippet_end = content[pos_byte..end_byte]
                .find(char::is_whitespace)
                .map(|i| pos_byte + i)
                .unwrap_or(end_byte);

            let mut s = String::new();
            if snippet_start > 0 {
                s.push_str("...");
            }
            s.push_str(content[snippet_start..snippet_end].trim());
            if snippet_end < content.len() {
                s.push_str("...");
            }
            s
        } else {
            let end_char = min(80, char_offsets.len());
            let end_byte = char_offsets.get(end_char).copied().unwrap_or(content.len());
            let snippet_end = content[..end_byte]
                .rfind(char::is_whitespace)
                .unwrap_or(end_byte);
            format!("{}...", content[..snippet_end].trim())
        }
    }
}
