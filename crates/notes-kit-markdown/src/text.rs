use crate::ast::{self, Element, Kind, Node};

pub fn text_of(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(t) | Node::Code(t) => out.push_str(t),
            Node::Html(_) | Node::Rule | Node::Task(_) | Node::FootnoteRef(_) => {}
            Node::SoftBreak | Node::HardBreak => out.push(' '),
            Node::Element(e) => {
                if let Kind::Link { wiki: true, labelled: false, dest } = &e.kind {
                    out.push_str(dest);
                    continue;
                }
                text_of(&e.children, out);
                if matches!(
                    e.kind,
                    Kind::Paragraph
                        | Kind::Heading(_)
                        | Kind::Item
                        | Kind::TableRow
                        | Kind::TableCell
                        | Kind::CodeBlock(_)
                        | Kind::BlockQuote
                        | Kind::Metadata
                ) {
                    out.push(' ');
                }
            }
        }
    }
}

pub fn element_text(e: &Element) -> String {
    let mut out = String::new();
    text_of(&e.children, &mut out);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn special_strings(text: &str) -> String {
    text.replace("---", "\u{2014}").replace("--", "\u{2013}")
}

pub fn plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    text_of(&ast::parse(s), &mut out);
    special_strings(&out).split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn reading_time(content: &str) -> usize {
    content.split_whitespace().count().div_ceil(200).max(1)
}

pub fn heading_slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_dash = true;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}

pub fn first_paragraph(nodes: &[Node]) -> Option<&Element> {
    nodes
        .iter()
        .take_while(|n| !ast::is_heading(n))
        .find_map(|n| match n {
            Node::Element(e @ Element { kind: Kind::Paragraph, .. }) => Some(e),
            _ => None,
        })
}

pub fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    let cut = match cut.rfind(' ') {
        Some(i) if i > max / 2 => cut[..i].to_string(),
        _ => cut,
    };
    format!("{}\u{2026}", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        assert_eq!(plain("see [example](https://x.com) and **bold** `code`"), "see example and bold code");
        assert_eq!(plain("[[h-02]] and [[rome|ROME]]"), "h-02 and ROME");
        assert_eq!(plain("- 2026-10-02 -- Reported on "), "2026-10-02 \u{2013} Reported on");
    }

    #[test]
    fn slugs() {
        assert_eq!(heading_slug("Why it matters?"), "why-it-matters");
        assert_eq!(heading_slug("  Open   questions "), "open-questions");
    }
}
