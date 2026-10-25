use leptos::prelude::*;

use crate::ast::{self, Element, Kind, Node};
use crate::format::is_wikilink_target;
use crate::render_config::RenderContext;
use crate::text::{element_text, heading_slug, special_strings, text_of};

pub fn render_markdown_document(content: &str, ctx: &RenderContext) -> impl IntoView {
    let nodes = ast::body_nodes(content);
    let views = render_children(&nodes, ctx);
    view! { <main>{views}</main> }
}

pub fn render_children(nodes: &[Node], ctx: &RenderContext) -> Vec<AnyView> {
    nodes.iter().filter_map(|n| render_node(n, ctx)).collect()
}

pub fn render_node(node: &Node, ctx: &RenderContext) -> Option<AnyView> {
    Some(match node {
        Node::Text(t) => special_strings(t).into_any(),
        Node::Code(t) => view! { <code>{t.clone()}</code> }.into_any(),
        Node::Html(t) => view! { <span inner_html=t.clone()></span> }.into_any(),
        Node::SoftBreak => "\n".into_any(),
        Node::HardBreak => view! { <br/> }.into_any(),
        Node::Rule => view! { <hr/> }.into_any(),
        Node::Task(done) => {
            let text = if *done { "[x]" } else { "[ ]" };
            view! { <span class="task-marker">{text}</span>" " }.into_any()
        }
        Node::FootnoteRef(label) => {
            let href = format!("#fn-{}", heading_slug(label));
            view! { <sup class="footnote-ref"><a href=href>{label.clone()}</a></sup> }.into_any()
        }
        Node::Element(e) => render_element(e, ctx)?,
    })
}

pub fn render_element(e: &Element, ctx: &RenderContext) -> Option<AnyView> {
    let cfg = &ctx.config;
    let custom = match &e.kind {
        Kind::Paragraph => cfg.paragraph.as_ref(),
        Kind::Heading(_) => cfg.heading.as_ref(),
        Kind::BlockQuote => cfg.block_quote.as_ref(),
        Kind::CodeBlock(_) => cfg.code_block.as_ref(),
        Kind::List(_) => cfg.list.as_ref(),
        Kind::Item => cfg.list_item.as_ref(),
        Kind::Table => cfg.table.as_ref(),
        Kind::Emphasis => cfg.emphasis.as_ref(),
        Kind::Strong => cfg.strong.as_ref(),
        Kind::Link { .. } => cfg.link.as_ref(),
        Kind::Image { .. } => cfg.image.as_ref(),
        _ => None,
    };
    if let Some(f) = custom {
        return Some(f(e, ctx));
    }
    Some(match &e.kind {
        Kind::Metadata => return None,
        Kind::Paragraph => default_render_paragraph(e, ctx),
        Kind::Heading(_) => default_render_heading(e, ctx),
        Kind::BlockQuote => default_render_block_quote(e, ctx),
        Kind::CodeBlock(_) => default_render_code_block(e, ctx),
        Kind::List(_) => default_render_list(e, ctx),
        Kind::Item => default_render_list_item(e, ctx),
        Kind::Footnote(label) => {
            let id = format!("fn-{}", heading_slug(label));
            let children = render_children(&e.children, ctx);
            view! { <div class="footnote" id=id><span class="footnote-label">{label.clone()}</span>{children}</div> }
                .into_any()
        }
        Kind::Table => default_render_table(e, ctx),
        Kind::TableHead | Kind::TableRow | Kind::TableCell | Kind::Other => {
            render_children(&e.children, ctx).into_any()
        }
        Kind::Emphasis => view! { <em>{render_children(&e.children, ctx)}</em> }.into_any(),
        Kind::Strong => view! { <strong>{render_children(&e.children, ctx)}</strong> }.into_any(),
        Kind::Strikethrough => view! { <s>{render_children(&e.children, ctx)}</s> }.into_any(),
        Kind::Link { .. } => default_render_link(e, ctx),
        Kind::Image { .. } => default_render_image(e, ctx),
    })
}

pub fn default_render_paragraph(e: &Element, ctx: &RenderContext) -> AnyView {
    let children = render_children(&e.children, ctx);
    view! { <p>{children}</p> }.into_any()
}

pub fn default_render_heading(e: &Element, ctx: &RenderContext) -> AnyView {
    let children = render_children(&e.children, ctx);
    let slug = heading_slug(&element_text(e));
    match e.kind {
        Kind::Heading(1) => view! { <h1 id=slug>{children}</h1> }.into_any(),
        Kind::Heading(2) => view! { <h2 id=slug>{children}</h2> }.into_any(),
        Kind::Heading(3) => view! { <h3 id=slug>{children}</h3> }.into_any(),
        Kind::Heading(4) => view! { <h4 id=slug>{children}</h4> }.into_any(),
        Kind::Heading(5) => view! { <h5 id=slug>{children}</h5> }.into_any(),
        _ => view! { <h6 id=slug>{children}</h6> }.into_any(),
    }
}

pub fn default_render_block_quote(e: &Element, ctx: &RenderContext) -> AnyView {
    let children = render_children(&e.children, ctx);
    view! { <blockquote>{children}</blockquote> }.into_any()
}

pub fn default_render_code_block(e: &Element, _ctx: &RenderContext) -> AnyView {
    let mut text = String::new();
    text_of(&e.children, &mut text);
    let lang = match &e.kind {
        Kind::CodeBlock(l) => l.clone(),
        _ => String::new(),
    };
    let class = (!lang.is_empty()).then(|| format!("language-{lang}"));
    view! { <pre><code class=class>{text}</code></pre> }.into_any()
}

pub fn default_render_list(e: &Element, ctx: &RenderContext) -> AnyView {
    let items = render_children(&e.children, ctx);
    match e.kind {
        Kind::List(Some(n)) => view! { <ol start=n>{items}</ol> }.into_any(),
        _ => view! { <ul>{items}</ul> }.into_any(),
    }
}

pub fn is_block(node: &Node) -> bool {
    matches!(
        node,
        Node::Element(Element {
            kind: Kind::Paragraph | Kind::List(_) | Kind::BlockQuote | Kind::CodeBlock(_) | Kind::Table | Kind::Heading(_),
            ..
        })
    )
}

pub fn split_item(item: &Element) -> (&[Node], &[Node]) {
    let split = item.children.iter().position(is_block).unwrap_or(item.children.len());
    item.children.split_at(split)
}

pub fn default_render_list_item(e: &Element, ctx: &RenderContext) -> AnyView {
    let ctx = ctx.inside_item();
    let children = render_children(&e.children, &ctx);
    view! { <li>{children}</li> }.into_any()
}

pub fn first_line(p: &Element) -> (String, usize) {
    let mut text = String::new();
    let mut i = 0;
    while let Some(node) = p.children.get(i) {
        match node {
            Node::SoftBreak | Node::HardBreak => break,
            Node::Text(t) | Node::Code(t) => text.push_str(t),
            Node::Element(e) => text_of(&e.children, &mut text),
            _ => {}
        }
        i += 1;
    }
    (text, i)
}

pub fn callout_of(quote: &Element) -> Option<(String, String)> {
    let Some(Node::Element(p)) = quote.children.first() else { return None };
    if p.kind != Kind::Paragraph {
        return None;
    }
    let (line, _) = first_line(p);
    let rest = line.trim_start().strip_prefix("[!")?;
    let end = rest.find(']')?;
    let kind = rest[..end].trim().to_string();
    let title = rest[end + 1..].trim().to_string();
    Some((kind, title))
}

pub fn callout_body(quote: &Element, ctx: &RenderContext) -> Vec<AnyView> {
    let mut body: Vec<AnyView> = Vec::new();
    for (idx, child) in quote.children.iter().enumerate() {
        if idx == 0 {
            if let Node::Element(p) = child {
                let (_, line_end) = first_line(p);
                let rest = &p.children[(line_end + 1).min(p.children.len())..];
                if !rest.is_empty() {
                    let views = render_children(rest, ctx);
                    body.push(view! { <p>{views}</p> }.into_any());
                }
            }
            continue;
        }
        body.extend(render_node(child, ctx));
    }
    body
}

pub fn table_rows(table: &Element) -> (Option<&Element>, Vec<&Element>) {
    let mut head = None;
    let mut body = Vec::new();
    for child in &table.children {
        let Node::Element(e) = child else { continue };
        match e.kind {
            Kind::TableHead => {
                if !element_text(e).trim().is_empty() {
                    head = Some(e);
                }
            }
            Kind::TableRow => body.push(e),
            _ => {}
        }
    }
    (head, body)
}

pub fn table_cells(row: &Element) -> Vec<&Element> {
    row.children
        .iter()
        .filter_map(|c| match c {
            Node::Element(cell @ Element { kind: Kind::TableCell, .. }) => Some(cell),
            _ => None,
        })
        .collect()
}

pub fn default_render_table(e: &Element, ctx: &RenderContext) -> AnyView {
    let ctx = ctx.inside_table();
    let (head, body) = table_rows(e);
    let head_view = head.map(|h| {
        let cells: Vec<AnyView> = table_cells(h)
            .into_iter()
            .map(|c| view! { <th scope="col">{render_children(&c.children, &ctx)}</th> }.into_any())
            .collect();
        view! { <thead><tr>{cells}</tr></thead> }
    });
    let rows: Vec<AnyView> = body
        .into_iter()
        .map(|r| {
            let cells: Vec<AnyView> = table_cells(r)
                .into_iter()
                .map(|c| view! { <td>{render_children(&c.children, &ctx)}</td> }.into_any())
                .collect();
            view! { <tr>{cells}</tr> }.into_any()
        })
        .collect();
    view! { <table>{head_view}<tbody>{rows}</tbody></table> }.into_any()
}

pub struct ResolvedLink {
    pub href: String,
    pub is_accessible: bool,
    pub is_note: bool,
    pub is_file: bool,
    pub is_image: bool,
    pub is_video: bool,
    pub is_audio: bool,
}

pub fn is_image_extension(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    matches!(
        lower.rsplit('.').next(),
        Some("webp" | "jpg" | "jpeg" | "png" | "gif" | "svg" | "avif" | "bmp" | "tiff")
    )
}

fn is_video_extension(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    matches!(lower.rsplit('.').next(), Some("mp4" | "webm" | "mov" | "m4v" | "ogv"))
}

fn is_audio_extension(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    matches!(lower.rsplit('.').next(), Some("mp3" | "wav" | "ogg" | "m4a" | "flac" | "opus"))
}

pub fn is_external(target: &str) -> bool {
    target.starts_with('#')
        || target.starts_with("//")
        || target.split_once(':').is_some_and(|(scheme, _)| {
            !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        })
}

pub fn asset_url(ctx: &RenderContext, rel: &str) -> String {
    let mut parts: Vec<&str> = if rel.starts_with('/') {
        Vec::new()
    } else {
        ctx.note_dir.split('/').filter(|s| !s.is_empty()).collect()
    };
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    format!("{}/{}", ctx.assets_prefix, parts.join("/"))
}

pub fn resolve_link(e: &Element, ctx: &RenderContext) -> ResolvedLink {
    let (wiki, dest) = match &e.kind {
        Kind::Link { wiki, dest, .. } => (*wiki, dest.as_str()),
        Kind::Image { dest } => (false, dest.as_str()),
        _ => (false, ""),
    };
    let dest = dest.strip_prefix("denote:").unwrap_or(dest);
    if wiki && is_wikilink_target(dest) {
        let accessible = ctx.accessible_ids.is_empty() || ctx.accessible_ids.contains(dest);
        if let Some(filename) = ctx.id_map.get(dest) {
            return ResolvedLink {
                href: format!("{}/{filename}", ctx.notes_prefix),
                is_accessible: accessible,
                is_note: true,
                is_file: false,
                is_image: false,
                is_video: false,
                is_audio: false,
            };
        }
        if let Some(asset) = ctx.asset_map.get(dest) {
            let href = format!("{}/{asset}", ctx.assets_prefix);
            return ResolvedLink {
                is_image: is_image_extension(&href),
                is_video: is_video_extension(&href),
                is_audio: is_audio_extension(&href),
                href,
                is_accessible: accessible,
                is_note: false,
                is_file: true,
            };
        }
        return ResolvedLink {
            href: dest.to_string(),
            is_accessible: accessible,
            is_note: true,
            is_file: false,
            is_image: false,
            is_video: false,
            is_audio: false,
        };
    }
    let dest = dest.strip_prefix("file:").unwrap_or(dest);
    let (href, is_file) = if is_external(dest) {
        (dest.to_string(), false)
    } else {
        (asset_url(ctx, dest), true)
    };
    ResolvedLink {
        is_image: is_image_extension(&href),
        is_video: is_video_extension(&href),
        is_audio: is_audio_extension(&href),
        href,
        is_accessible: true,
        is_note: false,
        is_file,
    }
}

pub fn link_label(e: &Element, ctx: &RenderContext) -> AnyView {
    match &e.kind {
        Kind::Link { wiki: true, labelled: false, dest } => {
            let dest = dest.strip_prefix("denote:").unwrap_or(dest).to_string();
            dest.into_any()
        }
        _ if e.children.is_empty() => match &e.kind {
            Kind::Link { dest, .. } | Kind::Image { dest } => dest.clone().into_any(),
            _ => ().into_any(),
        },
        _ => render_children(&e.children, ctx).into_any(),
    }
}

pub fn default_render_link(e: &Element, ctx: &RenderContext) -> AnyView {
    let resolved = resolve_link(e, ctx);
    if !resolved.is_accessible {
        return view! { <span>{link_label(e, ctx)}</span> }.into_any();
    }
    if resolved.is_image {
        let alt = element_text(e);
        return view! { <img src=resolved.href alt=alt loading="lazy" /> }.into_any();
    }
    if resolved.is_video {
        return view! { <video controls preload="metadata" src=resolved.href></video> }.into_any();
    }
    if resolved.is_audio {
        return view! { <audio controls preload="metadata" src=resolved.href></audio> }.into_any();
    }
    let external = resolved.href.starts_with("http://") || resolved.href.starts_with("https://");
    let class = if resolved.is_note { Some("note-link") } else { None };
    view! {
        <a href=resolved.href class=class rel=external.then_some("noopener")>{link_label(e, ctx)}</a>
    }
    .into_any()
}

pub fn default_render_image(e: &Element, ctx: &RenderContext) -> AnyView {
    let resolved = resolve_link(e, ctx);
    let alt = element_text(e);
    view! { <img src=resolved.href alt=alt loading="lazy" /> }.into_any()
}

pub fn is_image_paragraph(p: &Element) -> bool {
    let mut images = 0;
    for child in &p.children {
        match child {
            Node::Element(Element { kind: Kind::Image { dest }, .. }) if is_image_extension(dest) => images += 1,
            Node::Text(t) if t.trim().is_empty() => {}
            Node::SoftBreak | Node::HardBreak => {}
            _ => return false,
        }
    }
    images > 0
}
