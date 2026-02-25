use leptos::prelude::*;
use notes_kit_org::render_config::{RenderConfig, RenderContext};
use notes_kit_org::renderer::{
    default_render_headline, render_element, render_section_filtered,
};
use notes_kit_org::task_helpers::{is_planning_text, sum_clock_minutes};
use orgize::ast::Headline;
use orgize::SyntaxKind;

pub fn task_render_config() -> RenderConfig {
    RenderConfig::default().headline(render_task_headline)
}

fn render_task_headline(headline: &Headline, ctx: &RenderContext) -> AnyView {
    if !headline.is_done() {
        return default_render_headline(headline, ctx);
    }

    let total_minutes = sum_clock_minutes(headline);

    let title_views: Vec<AnyView> = headline
        .title()
        .filter_map(|elem| render_element(elem, ctx))
        .collect();

    let clock_badge = (total_minutes > 0).then(|| {
        let time_str = format!(
            "\u{1F559} {}:{:02}",
            total_minutes / 60,
            total_minutes % 60
        );
        view! { <span class="onk-task-clock">{time_str}</span> }
    });

    let section_view = headline.section().map(|s| {
        render_section_filtered(&s, ctx, |elem| {
            elem.kind() != SyntaxKind::DRAWER
                && !(elem.kind() == SyntaxKind::PARAGRAPH && is_planning_text(&elem.to_string()))
        })
    });

    let has_body = section_view.is_some();

    let child_headlines: Vec<AnyView> = headline
        .headlines()
        .map(|h| render_task_headline(&h, ctx))
        .collect();

    view! {
        <div class="onk-task-card">
            <div class="onk-task-header">
                <span class="onk-task-check">{"\u{2713}"}</span>
                <span class="onk-task-title">{title_views}</span>
                {clock_badge}
            </div>
            {has_body.then(|| view! {
                <div class="onk-task-body">{section_view}</div>
            })}
            {(!child_headlines.is_empty()).then(|| view! {
                <div class="onk-task-children">{child_headlines}</div>
            })}
        </div>
    }
    .into_any()
}

#[component]
pub fn App() -> impl IntoView {
    provide_context(task_render_config());
    view! { <notes_kit_app::DefaultApp /> }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
