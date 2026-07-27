use leptos::prelude::*;
use notes_kit_core::models::{Asset, Note};

use crate::server::assets::get_all_assets;
use crate::server::notes::{get_corpus_info, get_notes_summary};

/// Two-tier note data.
///
/// * `all_notes` — the serialized SSR resource: the body-less summary
///   index (metadata, excerpt, reading time, link ids, properties).
///   Grant-scoped. This is what every page renders from at SSR time.
/// * `corpus` — the full corpus (bodies + highlights), backfilled
///   client-side in the background: anonymous sessions fetch the shared
///   content-addressed snapshot (`/data/corpus/<hash>.json`, immutable,
///   HTTP-cacheable), authenticated sessions the grant-scoped server fn.
///   `None` on the server and until the backfill lands — consumers that
///   need bodies must degrade gracefully (or read
///   [`NotesContext::notes_with_bodies`], which falls back to
///   `all_notes`).
///
/// NOTE: `Resource`s serialize positionally — server and client must
/// create the same resources in the same order, so everything here is
/// created unconditionally.
#[derive(Clone, Copy)]
pub struct NotesContext {
    pub version: RwSignal<u64>,
    pub all_notes: Resource<Result<Vec<Note>, ServerFnError>>,
    pub all_assets: Resource<Result<Vec<Asset>, ServerFnError>>,
    pub corpus_info: Resource<Result<crate::server::notes::CorpusInfo, ServerFnError>>,
    pub corpus: RwSignal<Option<Vec<Note>>>,
}

impl NotesContext {
    pub fn bump_version(&self) {
        self.version.update(|v| *v = v.wrapping_add(1));
    }

    /// The best notes available right now: the backfilled full corpus if
    /// it has landed, else whatever the serialized resource holds.
    /// Reactive on both sources. Consumers that need bodies (search,
    /// previews) should read this and treat missing `content` as
    /// "not yet available".
    pub async fn notes_with_bodies(&self) -> Result<Vec<Note>, ServerFnError> {
        let serialized = self.all_notes.await?;
        Ok(match self.corpus.get() {
            Some(corpus) => corpus,
            None => serialized,
        })
    }

    /// Non-async variant of [`Self::notes_with_bodies`] for `.get()`-style
    /// consumers (memos, event handlers).
    pub fn notes_with_bodies_now(&self) -> Option<Vec<Note>> {
        if let Some(corpus) = self.corpus.get() {
            return Some(corpus);
        }
        self.all_notes.get().and_then(|r| r.ok())
    }
}

#[component]
pub fn QueryProvider(children: Children) -> impl IntoView {
    let version = RwSignal::new(0u64);

    // THE payoff of the two-tier design: what serializes into every SSR
    // response is the summary index, not the corpus.
    let all_notes = Resource::new(move || version.get(), |_| get_notes_summary());

    let all_assets = Resource::new(move || version.get(), |_| get_all_assets());

    let corpus_info = Resource::new(move || version.get(), |_| get_corpus_info());

    let corpus = RwSignal::new(None);

    let ctx = NotesContext {
        version,
        all_notes,
        all_assets,
        corpus_info,
        corpus,
    };

    provide_context(ctx);

    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;

        // Set by the initial backfill; SSE-triggered refreshes reuse it.
        let authed = StoredValue::new(false);

        let window = web_sys::window().unwrap();
        let cb = Closure::once(Box::new(move || {
            let es = web_sys::EventSource::new("/api/events/notes").unwrap();
            let on_msg = Closure::wrap(Box::new(move |ev: web_sys::MessageEvent| {
                ctx.bump_version();
                // Payload is the server's VersionInfo JSON; older servers
                // send a bare "changed", which parses to None and falls
                // back to the server fn.
                let snapshot_hash = ev
                    .data()
                    .as_string()
                    .and_then(|d| serde_json::from_str::<backfill::SseVersion>(&d).ok())
                    .and_then(|v| v.snapshot_hash);
                leptos::task::spawn_local(backfill::backfill_corpus(
                    ctx,
                    authed.get_value(),
                    snapshot_hash,
                ));
            }) as Box<dyn Fn(web_sys::MessageEvent)>);
            es.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
            on_msg.forget();
            let on_err = Closure::wrap(Box::new(move |_: web_sys::Event| {
                web_sys::console::warn_1(&"[sse] connection lost, reconnecting...".into());
            }) as Box<dyn Fn(web_sys::Event)>);
            es.set_onerror(Some(on_err.as_ref().unchecked_ref()));
            on_err.forget();
            std::mem::forget(es);

            // Initial corpus backfill: restores full-text search, hover
            // previews with bodies, and offline access to every note.
            leptos::task::spawn_local(async move {
                let info = ctx.corpus_info.await.unwrap_or_else(|_| {
                    crate::server::notes::CorpusInfo {
                        snapshot_hash: None,
                        authed: false,
                    }
                });
                authed.set_value(info.authed);
                backfill::backfill_corpus(ctx, info.authed, info.snapshot_hash).await;
            });
        }) as Box<dyn FnOnce()>);

        // requestIdleCallback when available
        let opts = web_sys::IdleRequestOptions::new();
        opts.set_timeout(3000);
        let scheduled =
            window.request_idle_callback_with_options(cb.as_ref().unchecked_ref(), &opts);
        if scheduled.is_err() {
            let already_loaded = window
                .document()
                .map(|d| d.ready_state() == "complete")
                .unwrap_or(false);
            if already_loaded {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    0,
                );
            } else {
                let _ =
                    window.add_event_listener_with_callback("load", cb.as_ref().unchecked_ref());
            }
        }
        cb.forget();
    }

    children()
}

#[cfg(feature = "hydrate")]
mod backfill {
    use super::NotesContext;
    use leptos::prelude::Set;
    use notes_kit_core::models::Note;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    /// Client-side mirror of notes-kit-server's SSE `VersionInfo` payload.
    #[derive(serde::Deserialize)]
    pub struct SseVersion {
        #[allow(dead_code)]
        pub content_hash: u64,
        pub snapshot_hash: Option<String>,
    }

    async fn sleep_ms(ms: i32) {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            if let Some(w) = web_sys::window() {
                let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
            }
        });
        let _ = JsFuture::from(promise).await;
    }

    async fn fetch_text(url: &str) -> Option<String> {
        let window = web_sys::window()?;
        let response = JsFuture::from(window.fetch_with_str(url)).await.ok()?;
        let response: web_sys::Response = response.dyn_into().ok()?;
        if !response.ok() {
            return None;
        }
        JsFuture::from(response.text().ok()?)
            .await
            .ok()?
            .as_string()
    }

    async fn fetch_once(authed: bool, snapshot_hash: Option<&str>) -> Option<Vec<Note>> {
        if authed {
            // Grant-scoped and uncached: private notes must never route
            // through the shared snapshot.
            return crate::server::notes::get_all_notes().await.ok();
        }
        if let Some(hash) = snapshot_hash {
            let url = format!("/data/corpus/{hash}.json");
            if let Some(text) = fetch_text(&url).await {
                match serde_json::from_str(&text) {
                    Ok(notes) => return Some(notes),
                    Err(e) => web_sys::console::warn_1(
                        &format!("[corpus] snapshot parse failed: {e}").into(),
                    ),
                }
            }
        }
        // No snapshot advertised, it 404'd (hash raced a redeploy), or it
        // failed to parse — the server fn returns the same public corpus.
        crate::server::notes::get_all_notes().await.ok()
    }

    /// Fill `ctx.corpus`, retrying with backoff. The last good corpus is
    /// never dropped on failure — offline navigation keeps whatever data
    /// already landed.
    pub async fn backfill_corpus(ctx: NotesContext, authed: bool, snapshot_hash: Option<String>) {
        let mut delay = 2_000;
        for _ in 0..5 {
            if let Some(notes) = fetch_once(authed, snapshot_hash.as_deref()).await {
                ctx.corpus.set(Some(notes));
                return;
            }
            sleep_ms(delay).await;
            delay = (delay * 3).min(60_000);
        }
        web_sys::console::warn_1(
            &"[corpus] backfill failed after retries; staying on current data".into(),
        );
    }
}
