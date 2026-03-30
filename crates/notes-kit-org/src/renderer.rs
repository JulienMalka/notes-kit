use leptos::prelude::*;
use leptos_router::components::A;
use orgize::{
    ast::*,
    rowan::ast::AstNode,
    SyntaxElement, SyntaxKind,
};
use std::cmp::min;

use crate::render_config::RenderContext;

pub fn render_org_document(content: &str, ctx: &RenderContext) -> impl IntoView {
    let org = orgize::Org::parse(content);
    let document = org.document();

    let section_view = document.section().map(|s| render_section(&s, ctx).into_any());
    let headlines: Vec<AnyView> = document
        .headlines()
        .map(|h| render_headline(&h, ctx))
        .collect();

    view! {
        <main>
            {section_view}
            {headlines}
        </main>
    }
}

pub fn render_children(
    elements: impl Iterator<Item = SyntaxElement>,
    ctx: &RenderContext,
) -> Vec<AnyView> {
    elements.filter_map(|elem| render_element(elem, ctx)).collect()
}

pub fn render_element(element: SyntaxElement, ctx: &RenderContext) -> Option<AnyView> {
    match element {
        SyntaxElement::Node(node) => match node.kind() {
            SyntaxKind::HEADLINE => {
                Headline::cast(node).map(|h| render_headline(&h, ctx))
            }
            SyntaxKind::SECTION => {
                Section::cast(node).map(|s| render_section(&s, ctx))
            }
            SyntaxKind::PARAGRAPH => {
                Paragraph::cast(node).map(|p| render_paragraph(&p, ctx))
            }
            SyntaxKind::BOLD => {
                Bold::cast(node).map(|b| render_bold(&b, ctx))
            }
            SyntaxKind::ITALIC => {
                Italic::cast(node).map(|i| render_italic(&i, ctx))
            }
            SyntaxKind::STRIKE => {
                Strike::cast(node).map(|s| render_strike(&s, ctx))
            }
            SyntaxKind::UNDERLINE => {
                Underline::cast(node).map(|u| render_underline(&u, ctx))
            }
            SyntaxKind::CODE => {
                Code::cast(node).map(|c| render_code(&c, ctx))
            }
            SyntaxKind::VERBATIM => {
                Verbatim::cast(node).map(|v| render_verbatim(&v, ctx))
            }
            SyntaxKind::LIST => {
                List::cast(node).map(|l| render_list(&l, ctx))
            }
            SyntaxKind::LIST_ITEM => {
                ListItem::cast(node).map(|li| render_list_item(&li, ctx))
            }
            SyntaxKind::SOURCE_BLOCK => {
                SourceBlock::cast(node).map(|sb| render_source_block(&sb, ctx))
            }
            SyntaxKind::QUOTE_BLOCK => {
                QuoteBlock::cast(node).map(|qb| render_quote_block(&qb, ctx))
            }
            SyntaxKind::EXAMPLE_BLOCK => {
                ExampleBlock::cast(node).map(|eb| render_example_block(&eb, ctx))
            }
            SyntaxKind::CENTER_BLOCK => {
                CenterBlock::cast(node).map(|cb| {
                    let children = cb
                        .syntax()
                        .children()
                        .find(|n| n.kind() == SyntaxKind::BLOCK_CONTENT)
                        .map(|n| render_children(n.children_with_tokens(), ctx))
                        .unwrap_or_default();
                    view! { <div class="center">{children}</div> }.into_any()
                })
            }
            SyntaxKind::VERSE_BLOCK => {
                VerseBlock::cast(node).map(|vb| {
                    let children = vb
                        .syntax()
                        .children()
                        .find(|n| n.kind() == SyntaxKind::BLOCK_CONTENT)
                        .map(|n| render_children(n.children_with_tokens(), ctx))
                        .unwrap_or_default();
                    view! { <p class="verse">{children}</p> }.into_any()
                })
            }
            SyntaxKind::FN_REF => {
                FnRef::cast(node).map(|f| render_fn_ref(&f, ctx))
            }
            SyntaxKind::FN_DEF => {
                FnDef::cast(node).map(|f| render_fn_def(&f, ctx))
            }
            SyntaxKind::COMMENT_BLOCK | SyntaxKind::KEYWORD => None,
            SyntaxKind::LINK => {
                Link::cast(node).map(|l| render_link(&l, ctx))
            }
            SyntaxKind::ORG_TABLE => {
                OrgTable::cast(node).map(|t| render_org_table(&t, ctx))
            }
            SyntaxKind::SUPERSCRIPT => {
                Superscript::cast(node).map(|s| {
                    let children = render_children(s.syntax().children_with_tokens(), ctx);
                    view! { <sup>{children}</sup> }.into_any()
                })
            }
            SyntaxKind::SUBSCRIPT => {
                Subscript::cast(node).map(|s| {
                    let children = render_children(s.syntax().children_with_tokens(), ctx);
                    view! { <sub>{children}</sub> }.into_any()
                })
            }
            SyntaxKind::RULE => Some(view! { <hr/> }.into_any()),
            SyntaxKind::LINE_BREAK => Some(view! { <br/> }.into_any()),
            SyntaxKind::BLOCK_CONTENT | SyntaxKind::LIST_ITEM_CONTENT => {
                let children = render_children(node.children_with_tokens(), ctx);
                if children.is_empty() { None } else { Some(children.into_any()) }
            }
            _ => {
                let children = render_children(node.children_with_tokens(), ctx);
                if children.is_empty() { None } else { Some(children.into_any()) }
            }
        },
        SyntaxElement::Token(token) => {
            if token.kind() == SyntaxKind::TEXT {
                let text = token.text().to_string();
                Some(view! { {text} }.into_any())
            } else {
                None
            }
        }
    }
}

fn render_headline(headline: &Headline, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.headline {
        return custom(headline, ctx);
    }
    default_render_headline(headline, ctx)
}

pub fn default_render_headline(headline: &Headline, ctx: &RenderContext) -> AnyView {
    let title = render_headline_title_tag(headline, ctx);
    let section_view = headline.section().map(|s| render_section(&s, ctx));
    let child_headlines: Vec<AnyView> = headline
        .headlines()
        .map(|h| render_headline(&h, ctx))
        .collect();

    view! {
        {title}
        {section_view}
        {child_headlines}
    }
    .into_any()
}

pub fn render_headline_title_tag(headline: &Headline, ctx: &RenderContext) -> AnyView {
    let level = min(headline.level(), 6);
    let title_views: Vec<AnyView> = headline
        .title()
        .filter_map(|elem| render_element(elem, ctx))
        .collect();

    match level {
        1 => view! { <h1>{title_views}</h1> }.into_any(),
        2 => view! { <h2>{title_views}</h2> }.into_any(),
        3 => view! { <h3>{title_views}</h3> }.into_any(),
        4 => view! { <h4>{title_views}</h4> }.into_any(),
        5 => view! { <h5>{title_views}</h5> }.into_any(),
        _ => view! { <h6>{title_views}</h6> }.into_any(),
    }
}

pub fn render_section_filtered<F>(
    section: &Section,
    ctx: &RenderContext,
    filter: F,
) -> AnyView
where
    F: Fn(&SyntaxElement) -> bool,
{
    let children: Vec<AnyView> = section
        .syntax()
        .children_with_tokens()
        .filter(|elem| filter(elem))
        .filter_map(|elem| render_element(elem, ctx))
        .collect();
    view! { <section>{children}</section> }.into_any()
}

fn render_section(section: &Section, ctx: &RenderContext) -> AnyView {
    let children = render_children(section.syntax().children_with_tokens(), ctx);
    view! { <section>{children}</section> }.into_any()
}

fn render_paragraph(paragraph: &Paragraph, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.paragraph {
        return custom(paragraph, ctx);
    }
    let children = render_children(paragraph.syntax().children_with_tokens(), ctx);
    view! { <p>{children}</p> }.into_any()
}

fn render_bold(bold: &Bold, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.bold {
        return custom(bold, ctx);
    }
    let children = render_children(bold.syntax().children_with_tokens(), ctx);
    view! { <b>{children}</b> }.into_any()
}

fn render_italic(italic: &Italic, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.italic {
        return custom(italic, ctx);
    }
    let children = render_children(italic.syntax().children_with_tokens(), ctx);
    view! { <i>{children}</i> }.into_any()
}

fn render_strike(strike: &Strike, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.strike {
        return custom(strike, ctx);
    }
    let children = render_children(strike.syntax().children_with_tokens(), ctx);
    view! { <s>{children}</s> }.into_any()
}

fn render_underline(underline: &Underline, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.underline {
        return custom(underline, ctx);
    }
    let children = render_children(underline.syntax().children_with_tokens(), ctx);
    view! { <u>{children}</u> }.into_any()
}

fn render_code(code: &Code, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.code {
        return custom(code, ctx);
    }
    let children = render_children(code.syntax().children_with_tokens(), ctx);
    view! { <code>{children}</code> }.into_any()
}

fn render_verbatim(verbatim: &Verbatim, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.verbatim {
        return custom(verbatim, ctx);
    }
    let children = render_children(verbatim.syntax().children_with_tokens(), ctx);
    view! { <code>{children}</code> }.into_any()
}

fn render_list(list: &List, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.list {
        return custom(list, ctx);
    }
    let children = render_children(list.syntax().children_with_tokens(), ctx);
    if list.is_ordered() {
        view! { <ol>{children}</ol> }.into_any()
    } else if list.is_descriptive() {
        view! { <dl>{children}</dl> }.into_any()
    } else {
        view! { <ul>{children}</ul> }.into_any()
    }
}

fn render_list_item(list_item: &ListItem, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.list_item {
        return custom(list_item, ctx);
    }

    let parent_is_descriptive = list_item
        .syntax()
        .parent()
        .and_then(List::cast)
        .is_some_and(|l| l.is_descriptive());

    if parent_is_descriptive {
        let tag_views: Vec<AnyView> = list_item
            .tag()
            .filter_map(|elem| render_element(elem, ctx))
            .collect();
        let content_views = render_children(list_item.syntax().children_with_tokens(), ctx);
        view! {
            <dt>{tag_views}</dt>
            <dd>{content_views}</dd>
        }
        .into_any()
    } else {
        let children = render_children(list_item.syntax().children_with_tokens(), ctx);
        view! { <li>{children}</li> }.into_any()
    }
}

fn render_source_block(source_block: &SourceBlock, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.source_block {
        return custom(source_block, ctx);
    }
    let content = source_block.value();
    if let Some(language) = source_block.language() {
        let class = format!("language-{language}");
        view! { <pre><code class={class}>{content}</code></pre> }.into_any()
    } else {
        view! { <pre><code>{content}</code></pre> }.into_any()
    }
}

fn render_quote_block(quote_block: &QuoteBlock, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.quote_block {
        return custom(quote_block, ctx);
    }
    let children = quote_block
        .syntax()
        .children()
        .find(|n| n.kind() == SyntaxKind::BLOCK_CONTENT)
        .map(|n| render_children(n.children_with_tokens(), ctx))
        .unwrap_or_default();
    view! { <blockquote>{children}</blockquote> }.into_any()
}

fn render_example_block(example_block: &ExampleBlock, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.example_block {
        return custom(example_block, ctx);
    }
    let content: String = example_block
        .syntax()
        .children()
        .find(|n| n.kind() == SyntaxKind::BLOCK_CONTENT)
        .map(|n| n.text().to_string())
        .unwrap_or_default();
    view! { <pre class="example">{content}</pre> }.into_any()
}

pub struct ResolvedLink {
    pub href: String,
    pub is_accessible: bool,
    pub is_internal: bool,
}

pub fn resolve_link(path: &str, ctx: &RenderContext) -> ResolvedLink {
    let (href, is_accessible) = if let Some(denote_id) = path.strip_prefix("denote:") {
        let accessible = ctx.accessible_ids.is_empty() || ctx.accessible_ids.contains(denote_id);
        let resolved = ctx
            .id_map
            .get(denote_id)
            .map(|filename| format!("{}/{filename}", ctx.notes_prefix))
            .unwrap_or_else(|| path.to_string());
        (resolved, accessible)
    } else {
        (path.trim_start_matches("file:").to_string(), true)
    };

    let is_internal = href.starts_with('/') && !href.starts_with("//");

    ResolvedLink {
        href,
        is_accessible,
        is_internal,
    }
}

pub fn default_render_link(link: &Link, ctx: &RenderContext) -> AnyView {
    let resolved = resolve_link(&link.path(), ctx);

    if !resolved.is_accessible {
        if link.has_description() {
            let desc: Vec<AnyView> = link
                .description()
                .filter_map(|elem| render_element(elem, ctx))
                .collect();
            return view! { <span>{desc}</span> }.into_any();
        }
        return view! { <span></span> }.into_any();
    }

    if link.is_image() {
        return view! { <img src={resolved.href}/> }.into_any();
    }

    if link.has_description() {
        let desc: Vec<AnyView> = link
            .description()
            .filter_map(|elem| render_element(elem, ctx))
            .collect();
        if resolved.is_internal {
            view! { <A href={resolved.href}>{desc}</A> }.into_any()
        } else {
            view! { <a href={resolved.href}>{desc}</a> }.into_any()
        }
    } else {
        let display_text = resolved.href.clone();
        if resolved.is_internal {
            view! { <A href={resolved.href}>{display_text}</A> }.into_any()
        } else {
            view! { <a href={resolved.href}>{display_text}</a> }.into_any()
        }
    }
}

fn render_link(link: &Link, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.link {
        return custom(link, ctx);
    }
    default_render_link(link, ctx)
}

fn render_org_table(table: &OrgTable, ctx: &RenderContext) -> AnyView {
    if let Some(ref custom) = ctx.config.org_table {
        return custom(table, ctx);
    }

    let has_header = table.has_header();
    let all_rows: Vec<OrgTableRow> = table
        .syntax()
        .children()
        .filter_map(OrgTableRow::cast)
        .collect();

    let mut header_rows = Vec::new();
    let mut body_rows = Vec::new();

    if has_header {
        let mut found_rule = false;
        for row in all_rows {
            if !found_rule {
                if row.is_rule() {
                    found_rule = true;
                } else if row.is_standard() {
                    header_rows.push(row);
                }
            } else if !row.is_rule() {
                body_rows.push(row);
            }
        }
    } else {
        body_rows = all_rows.into_iter().filter(|r| !r.is_rule()).collect();
    }

    view! {
        <table>
            {if !header_rows.is_empty() {
                Some(view! {
                    <thead>
                        {header_rows.into_iter().map(|row| render_table_row(&row, ctx)).collect_view()}
                    </thead>
                }.into_any())
            } else {
                None
            }}
            {if !body_rows.is_empty() {
                Some(view! {
                    <tbody>
                        {body_rows.into_iter().map(|row| render_table_row(&row, ctx)).collect_view()}
                    </tbody>
                }.into_any())
            } else {
                None
            }}
        </table>
    }
    .into_any()
}

fn render_table_row(row: &OrgTableRow, ctx: &RenderContext) -> AnyView {
    let cells: Vec<AnyView> = row
        .syntax()
        .children()
        .filter_map(OrgTableCell::cast)
        .map(|cell| {
            let children = render_children(cell.syntax().children_with_tokens(), ctx);
            view! { <td>{children}</td> }.into_any()
        })
        .collect();
    view! { <tr>{cells}</tr> }.into_any()
}

/// Extract the label from a FnRef or FnDef node.
///
/// The syntax tree is: `L_BRACKET TEXT("fn") COLON TEXT(label) ...`
/// The label is the TEXT token right after the first COLON.
fn extract_fn_label(syntax: &orgize::SyntaxNode) -> String {
    let mut found_first_colon = false;
    for token in syntax.children_with_tokens() {
        if let SyntaxElement::Token(t) = token {
            if t.kind() == SyntaxKind::COLON && !found_first_colon {
                found_first_colon = true;
                continue;
            }
            if found_first_colon && t.kind() == SyntaxKind::TEXT {
                return t.text().to_string();
            }
        }
    }
    String::new()
}

/// Extract the inline definition from a FnRef `[fn:LABEL:definition]`.
///
/// The definition content comes after the second COLON.
fn extract_fn_inline_def(syntax: &orgize::SyntaxNode, ctx: &RenderContext) -> Option<Vec<AnyView>> {
    let mut colon_count = 0;
    let mut found_second_colon = false;
    let mut children = Vec::new();

    for elem in syntax.children_with_tokens() {
        match &elem {
            SyntaxElement::Token(t) if t.kind() == SyntaxKind::COLON => {
                colon_count += 1;
                if colon_count == 2 {
                    found_second_colon = true;
                    continue;
                }
            }
            _ => {}
        }
        if found_second_colon {
            // Skip the closing bracket
            if let SyntaxElement::Token(t) = &elem {
                if t.kind() == SyntaxKind::R_BRACKET {
                    continue;
                }
            }
            if let Some(v) = render_element(elem, ctx) {
                children.push(v);
            }
        }
    }

    if found_second_colon && !children.is_empty() {
        Some(children)
    } else {
        None
    }
}

/// Render a footnote reference `[fn:LABEL]` as a superscript link.
fn render_fn_ref(fn_ref: &FnRef, ctx: &RenderContext) -> AnyView {
    let label = extract_fn_label(fn_ref.syntax());
    let href = format!("#fn-{label}");
    let id = format!("fnref-{label}");
    let display = label.clone();

    // Check for inline definition
    if let Some(def_views) = extract_fn_inline_def(fn_ref.syntax(), ctx) {
        // Inline footnote: render the ref and a hidden definition
        let def_id = format!("fn-{label}");
        let back_href = format!("#fnref-{label}");
        view! {
            <sup class="onk-fn-ref" id=id>
                <a href=href>{display}</a>
            </sup>
            <aside class="onk-fn-inline-def" id=def_id>
                {def_views}
                " "
                <a href=back_href class="onk-fn-back">{"\u{21A9}"}</a>
            </aside>
        }.into_any()
    } else {
        view! {
            <sup class="onk-fn-ref" id=id>
                <a href=href>{display}</a>
            </sup>
        }.into_any()
    }
}

/// Render a footnote definition `[fn:LABEL] content` as a footnote entry.
fn render_fn_def(fn_def: &FnDef, ctx: &RenderContext) -> AnyView {
    let label = extract_fn_label(fn_def.syntax());
    let id = format!("fn-{label}");
    let back_href = format!("#fnref-{label}");

    // Extract raw content after "[fn:LABEL]", then re-parse as org to render links etc.
    let raw = fn_def.raw();
    let content_str = raw
        .find(']')
        .map(|i| raw[i + 1..].trim())
        .unwrap_or("")
        .to_string();

    // Re-parse the content as org-mode to handle links, formatting, etc.
    let content_views = if content_str.is_empty() {
        Vec::new()
    } else {
        let org = orgize::Org::parse(&content_str);
        let doc = org.document();
        render_children(doc.syntax().children_with_tokens(), ctx)
    };

    view! {
        <div class="onk-fn-def" id=id>
            <span class="onk-fn-def-label">{label}</span>
            <span class="onk-fn-def-content">{content_views}</span>
            " "
            <a href=back_href class="onk-fn-back">{"\u{21A9}"}</a>
        </div>
    }.into_any()
}
