use leptos::prelude::*;
use notes_kit_core::models::NotesConfig;

use crate::hooks::use_search_modal;

#[component]
pub fn SearchModal(
    #[prop(into)]
    show: Signal<bool>,
    on_close: Callback<()>,
) -> impl IntoView {
    let notes_config = StoredValue::new(use_context::<NotesConfig>().unwrap_or_default());
    let modal = use_search_modal(on_close);

    view! {
        <Show when=move || show.get()>
            <div class="onk-search-overlay" on:click=move |_| modal.close.run(())>
                <div class="onk-search-modal" on:click=|ev| ev.stop_propagation()>
                    <input
                        class="onk-search-input"
                        type="text"
                        placeholder="Search notes..."
                        autofocus=true
                        prop:value=move || modal.search.query.get()
                        on:input=move |ev| modal.search.set_query.set(event_target_value(&ev))
                        on:keydown=move |ev| modal.on_keydown.run(ev)
                    />
                    <Show when=move || modal.search.has_results()>
                        <ul class="onk-search-results">
                            {move || {
                                let selected = modal.search.selected_index.get();
                                modal.search.results.get().into_iter().enumerate().map(|(i, result)| {
                                    let path = notes_config.with_value(|nc| nc.note_url(&result.path));
                                    let title = result.title.clone().unwrap_or_else(|| result.path.clone());
                                    let snippet = result.snippet.clone();
                                    let is_selected = i == selected;
                                    view! {
                                        <li
                                            class="onk-search-result"
                                            class:onk-search-result-selected=is_selected
                                            on:click=move |_| modal.navigate.run(path.clone())
                                        >
                                            <span class="onk-search-result-title">{title}</span>
                                            <span class="onk-search-result-snippet">{snippet}</span>
                                        </li>
                                    }
                                }).collect_view()
                            }}
                        </ul>
                    </Show>
                </div>
            </div>
        </Show>
    }
}
