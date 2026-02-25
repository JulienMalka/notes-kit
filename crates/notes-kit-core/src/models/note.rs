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
}

impl Note {
    pub fn list_entry(path: String, filename: String, metadata: NoteMetadata) -> Self {
        Self {
            path,
            filename,
            content: None,
            metadata,
            effective_signature: None,
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
