use std::collections::{BTreeMap, BTreeSet};

use notes_kit_core::denote::DenoteFilename;
use notes_kit_core::models::{NoteId, NoteMetadata, SummaryFields};
use notes_kit_core::traits::NoteFormat;

use crate::ast::{self, Element, Kind, Node};
use crate::frontmatter::{self, is_meta_key, FrontMatter, Value};
use crate::text;

#[derive(Default)]
pub struct MarkdownFormat;

pub fn stem(filename: &str) -> &str {
    let name = filename.rsplit('/').next().unwrap_or(filename);
    name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name)
}

pub fn is_wikilink_target(target: &str) -> bool {
    !target.is_empty()
        && !target.contains(':')
        && !target.starts_with(['#', '/', '.', '*', '(', '['])
}

pub fn parse_wikilink(value: &str) -> Option<(String, Option<String>)> {
    let inner = value.trim().strip_prefix("[[")?.strip_suffix("]]")?;
    let (target, label) = match inner.split_once('|') {
        Some((t, l)) => (t.trim(), Some(l.trim().to_string())),
        None => (inner.trim(), None),
    };
    is_wikilink_target(target).then(|| (target.to_string(), label))
}

pub fn wikilinks_in_text(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    for piece in value.split("]]") {
        if let Some(start) = piece.find("[[") {
            if let Some((target, _)) = parse_wikilink(&format!("{}]]", &piece[start..])) {
                out.push(target);
            }
        }
    }
    out
}

pub fn link_targets(nodes: &[Node], fm: &FrontMatter) -> BTreeSet<String> {
    let mut links = BTreeSet::new();
    for (_, value) in &fm.fields {
        links.extend(wikilinks_in_text(&value.text()));
    }
    ast::walk(nodes, &mut |e| {
        if let Kind::Link { wiki: true, dest, .. } = &e.kind {
            let dest = dest.strip_prefix("denote:").unwrap_or(dest);
            if is_wikilink_target(dest) {
                links.insert(dest.to_string());
            }
        }
    });
    links
}

pub fn first_heading_title(nodes: &[Node]) -> Option<String> {
    nodes.iter().find_map(|n| match n {
        Node::Element(e @ Element { kind: Kind::Heading(_), .. }) => {
            Some(text::element_text(e)).filter(|t| !t.is_empty())
        }
        _ => None,
    })
}

pub fn excerpt(nodes: &[Node], max_chars: usize) -> String {
    let raw = text::first_paragraph(nodes)
        .map(text::element_text)
        .unwrap_or_default();
    text::clip(&text::special_strings(&raw), max_chars)
}

pub fn properties(fm: &FrontMatter) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut order = Vec::new();
    for (k, v) in &fm.fields {
        if is_meta_key(k) {
            continue;
        }
        let key = k.to_ascii_uppercase();
        order.push(key.clone());
        map.insert(key, v.text().trim().to_string());
    }
    if !order.is_empty() {
        map.insert("_ORDER".to_string(), order.join(","));
    }
    map
}

impl NoteFormat for MarkdownFormat {
    fn extract_metadata(&self, content: &str, filename: &str) -> NoteMetadata {
        let fm = frontmatter::parse(content);
        let denote = DenoteFilename::parse(filename);
        let id = match &denote {
            Some(d) => NoteId::new(d.id.as_str()),
            None => NoteId::new(stem(filename)),
        };
        let title = fm
            .scalar("title")
            .or_else(|| first_heading_title(&ast::body_nodes(content)))
            .or_else(|| denote.as_ref().map(|d| d.display_title()));
        let date = fm
            .scalar("date")
            .or_else(|| denote.as_ref().map(|d| d.id.date()));
        NoteMetadata {
            id: Some(id),
            title,
            date,
            tags: fm.get("tags").map(Value::items).unwrap_or_default(),
            note_type: fm
                .scalar("type")
                .or_else(|| denote.as_ref().and_then(|d| d.note_type.clone())),
            signature: denote
                .as_ref()
                .and_then(|d| d.signature.clone())
                .or_else(|| fm.scalar("access")),
        }
    }

    fn parse_id(&self, filename: &str) -> Option<NoteId> {
        Some(match DenoteFilename::parse(filename) {
            Some(d) => NoteId::new(d.id.as_str()),
            None => NoteId::new(stem(filename)),
        })
    }

    fn file_extension(&self) -> &str {
        "md"
    }

    fn summary_fields(&self, content: &str) -> SummaryFields {
        let fm = frontmatter::parse(content);
        let nodes = ast::body_nodes(content);
        let link_ids: Vec<NoteId> = link_targets(&nodes, &fm).into_iter().map(NoteId::new).collect();
        SummaryFields {
            excerpt: Some(excerpt(&nodes, 320)),
            reading_minutes: Some(text::reading_time(fm.body(content)).min(u16::MAX as usize) as u16),
            link_ids: (!link_ids.is_empty()).then_some(link_ids),
            properties: Some(properties(&fm)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "---\ntitle: Rank-one edits\ntags: [editing]\nid: EXP-07\ntests: \"[[h-02]]\"\n---\n\nLead with [[h-02]] and **bold**.\n\n## Log\n\n- 2026-10-02 — Reported on [[rome|ROME]].\n";

    #[test]
    fn metadata() {
        let m = MarkdownFormat.extract_metadata(NOTE, "exp-07.md");
        assert_eq!(m.id.unwrap().as_str(), "exp-07");
        assert_eq!(m.title.as_deref(), Some("Rank-one edits"));
        assert_eq!(m.tags, vec!["editing".to_string()]);
        let d = MarkdownFormat.extract_metadata("# Hi\n", "20240101T120000==internal--hello__note.md");
        assert_eq!(d.id.unwrap().as_str(), "20240101T120000");
        assert_eq!(d.title.as_deref(), Some("Hi"));
        assert_eq!(d.signature.as_deref(), Some("internal"));
        assert_eq!(d.date.as_deref(), Some("2024-01-01"));
    }

    #[test]
    fn summary() {
        let s = MarkdownFormat.summary_fields(NOTE);
        let props = s.properties.unwrap();
        assert_eq!(props.get("ID").map(String::as_str), Some("EXP-07"));
        assert_eq!(props.get("_ORDER").map(String::as_str), Some("ID,TESTS"));
        let links: Vec<&str> = s.link_ids.as_ref().unwrap().iter().map(|i| i.as_str()).collect();
        assert_eq!(links, vec!["h-02", "rome"]);
        assert!(s.excerpt.unwrap().starts_with("Lead with h-02 and bold."));
    }
}
