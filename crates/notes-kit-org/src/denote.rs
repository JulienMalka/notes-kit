use serde::{Deserialize, Serialize};
use std::borrow::Borrow;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DenoteId(String);

impl DenoteId {
    pub fn parse(s: &str) -> Option<Self> {
        if s.len() < 15 {
            return None;
        }
        let candidate = &s[..15];
        let bytes = candidate.as_bytes();

        if bytes[8] != b'T' {
            return None;
        }

        for (i, &b) in bytes.iter().enumerate() {
            if i == 8 {
                continue;
            }
            if !b.is_ascii_digit() {
                return None;
            }
        }

        Some(DenoteId(candidate.to_string()))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn date(&self) -> String {
        let s = &self.0;
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    }
}

impl std::fmt::Display for DenoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for DenoteId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for DenoteId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenoteFilename {
    pub id: DenoteId,
    pub signature: Option<String>,
    pub title: String,
    pub note_type: Option<String>,
    pub extension: String,
}

impl DenoteFilename {
    pub fn parse(filename: &str) -> Option<Self> {
        let dot_pos = filename.rfind('.')?;
        let extension = filename[dot_pos + 1..].to_string();
        let without_ext = &filename[..dot_pos];

        let id = DenoteId::parse(without_ext)?;
        let rest = &without_ext[15..];

        let (signature, after_sig) = if let Some(stripped) = rest.strip_prefix("==") {
            let sig_end = stripped.find("--").unwrap_or(stripped.len());
            (Some(stripped[..sig_end].to_string()), &stripped[sig_end..])
        } else {
            (None, rest)
        };

        let after_title_marker = after_sig.strip_prefix("--")?;

        let (title, note_type) = if let Some(pos) = after_title_marker.rfind("__") {
            (
                after_title_marker[..pos].to_string(),
                Some(after_title_marker[pos + 2..].to_string()),
            )
        } else {
            (after_title_marker.to_string(), None)
        };

        Some(Self {
            id,
            signature,
            title,
            note_type,
            extension,
        })
    }

    pub fn is_note(&self) -> bool {
        self.extension == "org"
    }

    pub fn from_path(path: &str) -> Option<Self> {
        let filename = path.rsplit('/').next().unwrap_or(path);
        Self::parse(filename)
    }

    pub fn display_title(&self) -> String {
        self.title.replace('-', " ")
    }

    pub fn to_filename(&self) -> String {
        let mut result = self.id.as_str().to_string();

        if let Some(ref sig) = self.signature {
            result.push_str("==");
            result.push_str(sig);
        }

        result.push_str("--");
        result.push_str(&self.title);

        if let Some(ref nt) = self.note_type {
            result.push_str("__");
            result.push_str(nt);
        }

        result.push('.');
        result.push_str(&self.extension);
        result
    }

    pub fn note_type_or_default(&self) -> &str {
        self.note_type.as_deref().unwrap_or("note")
    }
}

pub fn short_id_from_filename(filename: &str) -> &str {
    if let Some(pos) = filename.find("--") {
        &filename[..pos]
    } else {
        &filename[..filename.len().min(16)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_id() {
        let id = DenoteId::parse("20250107T123456").unwrap();
        assert_eq!(id.as_str(), "20250107T123456");
    }

    #[test]
    fn parse_id_from_longer_string() {
        let id = DenoteId::parse("20250107T123456--some-title").unwrap();
        assert_eq!(id.as_str(), "20250107T123456");
    }

    #[test]
    fn reject_short_string() {
        assert!(DenoteId::parse("2025010").is_none());
    }

    #[test]
    fn reject_missing_t_separator() {
        assert!(DenoteId::parse("12345678X123456").is_none());
    }

    #[test]
    fn reject_non_digit_chars() {
        assert!(DenoteId::parse("2025010aT123456").is_none());
    }

    #[test]
    fn date_extraction() {
        let id = DenoteId::parse("20250107T123456").unwrap();
        assert_eq!(id.date(), "2025-01-07");
    }

    #[test]
    fn display_trait() {
        let id = DenoteId::parse("20250107T123456").unwrap();
        assert_eq!(format!("{}", id), "20250107T123456");
    }

    #[test]
    fn parse_simple_filename() {
        let df = DenoteFilename::parse("20250107T123456--my-note.org").unwrap();
        assert_eq!(df.id.as_str(), "20250107T123456");
        assert_eq!(df.title, "my-note");
        assert!(df.signature.is_none());
        assert!(df.note_type.is_none());
    }

    #[test]
    fn parse_filename_with_type() {
        let df = DenoteFilename::parse("20250107T123456--my-note__literature.org").unwrap();
        assert_eq!(df.title, "my-note");
        assert_eq!(df.note_type.as_deref(), Some("literature"));
    }

    #[test]
    fn parse_filename_with_signature() {
        let df = DenoteFilename::parse("20250107T123456==private--secret-note.org").unwrap();
        assert_eq!(df.signature.as_deref(), Some("private"));
        assert_eq!(df.title, "secret-note");
    }

    #[test]
    fn parse_filename_with_all_components() {
        let df =
            DenoteFilename::parse("20250107T123456==phd--research-topic__literature.org").unwrap();
        assert_eq!(df.id.as_str(), "20250107T123456");
        assert_eq!(df.signature.as_deref(), Some("phd"));
        assert_eq!(df.title, "research-topic");
        assert_eq!(df.note_type.as_deref(), Some("literature"));
    }

    #[test]
    fn parse_non_org_extension() {
        let df = DenoteFilename::parse("20250107T123456--title.md").unwrap();
        assert_eq!(df.extension, "md");
        assert!(!df.is_note());
    }

    #[test]
    fn parse_image_extension() {
        let df = DenoteFilename::parse("20250107T123456--sunset-photo__photo.webp").unwrap();
        assert_eq!(df.extension, "webp");
        assert_eq!(df.title, "sunset-photo");
        assert_eq!(df.note_type.as_deref(), Some("photo"));
        assert!(!df.is_note());
    }

    #[test]
    fn reject_no_extension() {
        assert!(DenoteFilename::parse("20250107T123456--title").is_none());
    }

    #[test]
    fn reject_missing_title_separator() {
        assert!(DenoteFilename::parse("20250107T123456title.org").is_none());
    }

    #[test]
    fn from_path_extracts_filename() {
        let df =
            DenoteFilename::from_path("/notes/20250107T123456--my-note__note.org").unwrap();
        assert_eq!(df.id.as_str(), "20250107T123456");
        assert_eq!(df.title, "my-note");
    }

    #[test]
    fn display_title_replaces_hyphens() {
        let df = DenoteFilename::parse("20250107T123456--my-cool-note.org").unwrap();
        assert_eq!(df.display_title(), "my cool note");
    }

    #[test]
    fn to_filename_roundtrip() {
        let original = "20250107T123456==phd--research-topic__literature.org";
        let df = DenoteFilename::parse(original).unwrap();
        assert_eq!(df.to_filename(), original);
    }

    #[test]
    fn short_id_basic() {
        assert_eq!(short_id_from_filename("20250107T123456--my-note.org"), "20250107T123456");
    }

    #[test]
    fn short_id_no_separator() {
        assert_eq!(short_id_from_filename("shortname.org"), "shortname.org");
    }

    #[test]
    fn short_id_with_signature() {
        assert_eq!(
            short_id_from_filename("20250107T123456==priv--note.org"),
            "20250107T123456==priv"
        );
    }

    #[test]
    fn note_type_or_default() {
        let with_type = DenoteFilename::parse("20250107T123456--note__people.org").unwrap();
        assert_eq!(with_type.note_type_or_default(), "people");

        let without_type = DenoteFilename::parse("20250107T123456--note.org").unwrap();
        assert_eq!(without_type.note_type_or_default(), "note");
    }
}
