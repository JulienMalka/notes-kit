pub use pulldown_cmark;

pub mod ast;
pub mod format;
pub mod frontmatter;
pub mod render_config;
pub mod renderer;
pub mod text;

use leptos::prelude::*;

#[component]
pub fn MarkdownContent(#[prop(into)] content: String) -> impl IntoView {
    let ctx = use_context::<render_config::RenderContext>().unwrap_or_default();
    renderer::render_markdown_document(&content, &ctx)
}
