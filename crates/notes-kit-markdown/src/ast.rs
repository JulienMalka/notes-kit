use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag};

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Paragraph,
    Heading(u8),
    BlockQuote,
    CodeBlock(String),
    List(Option<u64>),
    Item,
    Footnote(String),
    Table,
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Link { wiki: bool, labelled: bool, dest: String },
    Image { dest: String },
    Metadata,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Text(String),
    Code(String),
    Html(String),
    SoftBreak,
    HardBreak,
    Rule,
    Task(bool),
    FootnoteRef(String),
    Element(Element),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub kind: Kind,
    pub children: Vec<Node>,
}

impl Element {
    pub fn is(&self, kind: &Kind) -> bool {
        &self.kind == kind
    }
}

pub fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_WIKILINKS
}

fn level_of(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn kind_of(tag: Tag) -> Kind {
    match tag {
        Tag::Paragraph => Kind::Paragraph,
        Tag::Heading { level, .. } => Kind::Heading(level_of(level)),
        Tag::BlockQuote(_) => Kind::BlockQuote,
        Tag::CodeBlock(CodeBlockKind::Fenced(lang)) => Kind::CodeBlock(lang.to_string()),
        Tag::CodeBlock(CodeBlockKind::Indented) => Kind::CodeBlock(String::new()),
        Tag::List(start) => Kind::List(start),
        Tag::Item => Kind::Item,
        Tag::FootnoteDefinition(label) => Kind::Footnote(label.to_string()),
        Tag::Table(_) => Kind::Table,
        Tag::TableHead => Kind::TableHead,
        Tag::TableRow => Kind::TableRow,
        Tag::TableCell => Kind::TableCell,
        Tag::Emphasis => Kind::Emphasis,
        Tag::Strong => Kind::Strong,
        Tag::Strikethrough => Kind::Strikethrough,
        Tag::Link { link_type, dest_url, .. } => match link_type {
            LinkType::WikiLink { has_pothole } => Kind::Link {
                wiki: true,
                labelled: has_pothole,
                dest: dest_url.to_string(),
            },
            _ => Kind::Link { wiki: false, labelled: true, dest: dest_url.to_string() },
        },
        Tag::Image { dest_url, .. } => Kind::Image { dest: dest_url.to_string() },
        Tag::MetadataBlock(_) => Kind::Metadata,
        _ => Kind::Other,
    }
}

pub fn parse(content: &str) -> Vec<Node> {
    let root = || Element { kind: Kind::Other, children: Vec::new() };
    let mut stack: Vec<Element> = vec![root()];
    for event in Parser::new_ext(content, options()) {
        let node = match event {
            Event::Start(tag) => {
                stack.push(Element { kind: kind_of(tag), children: Vec::new() });
                continue;
            }
            Event::End(_) => {
                let done = stack.pop().unwrap_or_else(root);
                if stack.is_empty() {
                    stack.push(root());
                }
                Node::Element(done)
            }
            Event::Text(t) => Node::Text(t.to_string()),
            Event::Code(t) => Node::Code(t.to_string()),
            Event::InlineMath(t) | Event::DisplayMath(t) => Node::Code(t.to_string()),
            Event::Html(t) | Event::InlineHtml(t) => Node::Html(t.to_string()),
            Event::FootnoteReference(t) => Node::FootnoteRef(t.to_string()),
            Event::SoftBreak => Node::SoftBreak,
            Event::HardBreak => Node::HardBreak,
            Event::Rule => Node::Rule,
            Event::TaskListMarker(done) => Node::Task(done),
        };
        if let Some(top) = stack.last_mut() {
            top.children.push(node);
        }
    }
    while stack.len() > 1 {
        let done = stack.pop().unwrap_or_else(root);
        if let Some(top) = stack.last_mut() {
            top.children.push(Node::Element(done));
        }
    }
    stack.pop().map(|e| e.children).unwrap_or_default()
}

pub fn body_nodes(content: &str) -> Vec<Node> {
    parse(content)
        .into_iter()
        .filter(|n| !matches!(n, Node::Element(Element { kind: Kind::Metadata, .. })))
        .collect()
}

pub fn walk<'a>(nodes: &'a [Node], f: &mut dyn FnMut(&'a Element)) {
    for node in nodes {
        if let Node::Element(e) = node {
            f(e);
            walk(&e.children, f);
        }
    }
}

pub fn is_heading(node: &Node) -> bool {
    matches!(node, Node::Element(Element { kind: Kind::Heading(_), .. }))
}

pub fn headings(nodes: &[Node]) -> Vec<(u8, &Element)> {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::Element(e @ Element { kind: Kind::Heading(l), .. }) => Some((*l, e)),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_shape() {
        let nodes = body_nodes("---\ntitle: T\n---\n\nLead [[a|A]].\n\n## Setup\n\n- a\n");
        assert!(matches!(&nodes[0], Node::Element(Element { kind: Kind::Paragraph, .. })));
        assert!(matches!(&nodes[1], Node::Element(Element { kind: Kind::Heading(2), .. })));
        let mut links = Vec::new();
        walk(&nodes, &mut |e| {
            if let Kind::Link { wiki: true, labelled, dest } = &e.kind {
                links.push((dest.clone(), *labelled));
            }
        });
        assert_eq!(links, vec![("a".to_string(), true)]);
    }
}
