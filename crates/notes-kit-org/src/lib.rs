pub mod date;
pub mod denote;
pub mod format;
pub mod helpers;
pub mod render_config;
pub mod renderer;
pub mod task_helpers;
pub mod text;

use leptos::prelude::*;

#[component]
pub fn OrgContent(
    #[prop(into)]
    content: String,
) -> impl IntoView {
    let ctx = use_context::<render_config::RenderContext>()
        .unwrap_or_default();
    renderer::render_org_document(&content, &ctx)
}
