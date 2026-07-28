//! Atom feed (`/feed.xml`).
//!
//! Built from the in-memory cache with anonymous grants only — feeds are
//! public artifacts, so the same privacy gate as the sitemap and the
//! corpus snapshot applies. Note bodies are rendered to plain HTML with
//! orgize's exporter (no app render config: feed readers want standard
//! anchors, not hover-preview widgets), then denote links are resolved to
//! absolute note URLs.

use notes_kit_core::models::Note;
use std::collections::HashMap;

/// Host-app configuration for the feed. `None` on `ServerConfig` disables
/// the route.
#[derive(Clone)]
pub struct FeedConfig {
    /// Absolute base URL, e.g. "https://example.com" (no trailing slash).
    pub base_url: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    /// `note_type` values included in the feed (e.g. `["blog"]`).
    pub note_types: Vec<String>,
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// First `YYYY-MM-DD` in the string (mirrors the sitemap's date handling).
fn extract_iso_date(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    for i in 0..=bytes.len() - 10 {
        let sb = &bytes[i..i + 10];
        if sb[4] == b'-'
            && sb[7] == b'-'
            && sb.iter().enumerate().all(|(j, b)| {
                if j == 4 || j == 7 {
                    true
                } else {
                    b.is_ascii_digit()
                }
            })
        {
            return Some(&s[i..i + 10]);
        }
    }
    None
}

/// Strip leading org metadata (keywords + property/logbook drawers) so the
/// exported HTML starts at the prose.
fn strip_org_header(content: &str) -> &str {
    let mut pos = 0;
    let mut in_drawer = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if in_drawer {
            pos += line.len() + 1;
            if trimmed.eq_ignore_ascii_case(":END:") {
                in_drawer = false;
            }
            continue;
        }
        if trimmed.starts_with("#+") || trimmed.is_empty() {
            pos += line.len() + 1;
            continue;
        }
        if trimmed.starts_with(":PROPERTIES:") || trimmed.starts_with(":LOGBOOK:") {
            in_drawer = true;
            pos += line.len() + 1;
            continue;
        }
        break;
    }
    if pos < content.len() {
        &content[pos..]
    } else {
        ""
    }
}

/// Render a note body to feed HTML: orgize export, then absolutize denote
/// links (via the id → path map) and site-relative hrefs/srcs.
fn body_html(content: &str, id_map: &HashMap<String, String>, base: &str) -> String {
    use notes_kit_org::orgize::export::HtmlExport;
    use notes_kit_org::orgize::rowan::ast::AstNode;
    use notes_kit_org::orgize::Org;

    let org = Org::parse(strip_org_header(content));
    let mut export = HtmlExport::default();
    export.render(org.document().syntax());
    let mut html = export.finish();

    for (id, path) in id_map {
        html = html.replace(
            &format!("href=\"denote:{id}\""),
            &format!("href=\"{base}/notes/{path}\""),
        );
    }
    html = html.replace("href=\"/", &format!("href=\"{base}/"));
    html = html.replace("src=\"/", &format!("src=\"{base}/"));
    html
}

/// Build the Atom document from the accessible notes.
pub(crate) fn build_atom(cfg: &FeedConfig, notes: &[Note]) -> String {
    let base = cfg.base_url.trim_end_matches('/');

    let id_map: HashMap<String, String> = notes
        .iter()
        .filter_map(|n| {
            n.metadata
                .id
                .as_ref()
                .map(|id| (id.as_str().to_string(), n.path.clone()))
        })
        .collect();

    let mut entries: Vec<&Note> = notes
        .iter()
        .filter(|n| n.signature() == "public")
        .filter(|n| {
            n.metadata
                .note_type
                .as_deref()
                .map(|t| cfg.note_types.iter().any(|w| w == t))
                .unwrap_or(false)
        })
        .collect();
    // Denote filenames start with the timestamp — descending = newest first.
    entries.sort_by(|a, b| b.filename.cmp(&a.filename));

    let feed_updated = entries
        .iter()
        .filter_map(|n| n.metadata.date.as_deref().and_then(extract_iso_date))
        .max()
        .map(|d| format!("{d}T00:00:00Z"))
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    let mut xml = String::with_capacity(64 * 1024);
    xml.push_str(r#"<?xml version="1.0" encoding="utf-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<feed xmlns="http://www.w3.org/2005/Atom">"#);
    xml.push('\n');
    xml.push_str(&format!("  <title>{}</title>\n", xml_escape(&cfg.title)));
    if let Some(subtitle) = &cfg.subtitle {
        xml.push_str(&format!(
            "  <subtitle>{}</subtitle>\n",
            xml_escape(subtitle)
        ));
    }
    xml.push_str(&format!("  <id>{base}/</id>\n"));
    xml.push_str(&format!(
        "  <link rel=\"alternate\" href=\"{base}/\"/>\n"
    ));
    xml.push_str(&format!(
        "  <link rel=\"self\" href=\"{base}/feed.xml\"/>\n"
    ));
    xml.push_str(&format!("  <updated>{feed_updated}</updated>\n"));
    xml.push_str(&format!(
        "  <author><name>{}</name></author>\n",
        xml_escape(&cfg.author)
    ));

    for note in entries {
        let url = format!("{base}/notes/{}", note.path);
        let title = note.display_title();
        let date = note
            .metadata
            .date
            .as_deref()
            .and_then(extract_iso_date)
            .map(|d| format!("{d}T00:00:00Z"))
            .unwrap_or_else(|| feed_updated.clone());
        let content = note
            .content
            .as_deref()
            .map(|c| body_html(c, &id_map, base))
            .unwrap_or_default();

        xml.push_str("  <entry>\n");
        xml.push_str(&format!("    <title>{}</title>\n", xml_escape(title)));
        xml.push_str(&format!("    <id>{}</id>\n", xml_escape(&url)));
        xml.push_str(&format!(
            "    <link rel=\"alternate\" href=\"{}\"/>\n",
            xml_escape(&url)
        ));
        xml.push_str(&format!("    <published>{date}</published>\n"));
        xml.push_str(&format!("    <updated>{date}</updated>\n"));
        xml.push_str(&format!(
            "    <content type=\"html\">{}</content>\n",
            xml_escape(&content)
        ));
        xml.push_str("  </entry>\n");
    }

    xml.push_str("</feed>\n");
    xml
}

/// Axum handler for `GET /feed.xml`.
pub async fn serve_feed(
    axum::Extension(state): axum::Extension<crate::state::AppState>,
    axum::Extension(cfg): axum::Extension<Option<FeedConfig>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let Some(cfg) = cfg else {
        return (http::StatusCode::NOT_FOUND, "feed not configured").into_response();
    };

    // Anonymous grants only — the feed is a shared public artifact.
    let grants = state.authz_policy.anonymous_grants();
    let notes = match state.repository.list_accessible(&grants).await {
        Ok(n) => n,
        Err(e) => {
            return (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("feed error: {e}"),
            )
                .into_response();
        }
    };

    let xml = build_atom(&cfg, &notes);
    let headers = [
        (
            http::header::CONTENT_TYPE,
            "application/atom+xml; charset=utf-8".to_string(),
        ),
        (
            http::header::CACHE_CONTROL,
            "public, max-age=3600".to_string(),
        ),
    ];
    (headers, xml).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use notes_kit_core::models::NoteMetadata;

    fn note(filename: &str, note_type: &str, sig: Option<&str>, content: &str) -> Note {
        let mut n = Note::list_entry(
            filename.to_string(),
            filename.to_string(),
            NoteMetadata {
                id: None,
                title: Some(filename.to_string()),
                date: Some("2026-01-15".to_string()),
                tags: Vec::new(),
                note_type: Some(note_type.to_string()),
                signature: sig.map(|s| s.to_string()),
            },
        );
        n.content = Some(content.to_string());
        n.effective_signature = Some(sig.unwrap_or("private").to_string());
        n
    }

    #[test]
    fn feed_filters_types_and_private_notes() {
        let cfg = FeedConfig {
            base_url: "https://example.com".into(),
            title: "T".into(),
            subtitle: None,
            author: "A".into(),
            note_types: vec!["blog".into()],
        };
        let notes = vec![
            note("b1.org", "blog", Some("public"), "Hello *world*"),
            note("b2.org", "blog", Some("private"), "SECRET"),
            note("t1.org", "talk", Some("public"), "talk body"),
        ];
        let xml = build_atom(&cfg, &notes);
        assert!(xml.contains("b1.org"));
        assert!(!xml.contains("SECRET"), "private note leaked into feed");
        assert!(!xml.contains("talk body"), "non-blog type included");
        assert!(xml.contains("2026-01-15T00:00:00Z"));
    }
}
