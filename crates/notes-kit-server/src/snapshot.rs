//! Public corpus snapshot.
//!
//! The full anonymous-accessible note corpus, serialized once per cache
//! version and served as a content-addressed, immutably-cacheable JSON
//! asset (`/data/corpus/<hash>.json`). Clients backfill their in-memory
//! corpus from it in the background instead of re-downloading the corpus
//! inside every SSR response.
//!
//! SECURITY INVARIANT: snapshots are shared, unauthenticated artifacts.
//! They must only ever be built from `AuthzPolicy::anonymous_grants()` —
//! never from a session's grants. Authenticated sessions fetch their
//! private notes through the grant-scoped, uncached server fns instead.

use bytes::Bytes;
use notes_kit_core::models::Note;
use std::io::Write;
use std::sync::{Arc, RwLock};

/// What the version watch channel carries: the grant-independent content
/// hash of the whole cache, plus the current public-snapshot hash. Sent
/// to clients as the SSE payload (JSON).
#[derive(Clone, Debug, serde::Serialize)]
pub struct VersionInfo {
    /// `DefaultRepository::global_version_hash()` over ALL notes —
    /// changes on any note edit, including private ones.
    pub content_hash: u64,
    /// Hash of the current public corpus snapshot, if one is built.
    pub snapshot_hash: Option<String>,
}

/// One immutable serialization of the public corpus, precompressed at
/// build time so requests are pure memory copies.
pub struct CorpusSnapshot {
    /// Hex content hash of `raw` — the URL path component.
    pub hash: String,
    pub raw: Bytes,
    pub br: Bytes,
    pub gz: Bytes,
}

/// Current + previous snapshot. The previous version stays available for
/// a grace window so a client that was told a hash just before a refresh
/// doesn't 404.
#[derive(Default, Clone)]
pub struct SnapshotStore(Arc<RwLock<SnapshotInner>>);

#[derive(Default)]
struct SnapshotInner {
    current: Option<Arc<CorpusSnapshot>>,
    previous: Option<Arc<CorpusSnapshot>>,
}

impl SnapshotStore {
    pub fn current_hash(&self) -> Option<String> {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .current
            .as_ref()
            .map(|s| s.hash.clone())
    }

    pub fn get(&self, hash: &str) -> Option<Arc<CorpusSnapshot>> {
        let inner = self.0.read().unwrap_or_else(|e| e.into_inner());
        for snap in [&inner.current, &inner.previous].into_iter().flatten() {
            if snap.hash == hash {
                return Some(Arc::clone(snap));
            }
        }
        None
    }

    /// Atomically swap in a new snapshot. Callers must publish BEFORE
    /// announcing the new hash (SSE / SSR), so no client is ever told a
    /// hash that 404s.
    pub fn publish(&self, snapshot: CorpusSnapshot) {
        let mut inner = self.0.write().unwrap_or_else(|e| e.into_inner());
        inner.previous = inner.current.take();
        inner.current = Some(Arc::new(snapshot));
    }
}

/// FNV-1a 64. Stable by definition — unlike `DefaultHasher`, whose
/// algorithm may change across Rust releases — so snapshot URLs survive
/// toolchain upgrades. Hashing the serialized bytes (rather than selected
/// fields) means the URL identifies exactly what is served, by
/// construction: any change to highlights, metadata, or serialization
/// shape changes the URL.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Serialize + precompress a corpus. CPU-heavy (brotli q11); run inside
/// `spawn_blocking`. Deterministic for a given corpus: notes are sorted
/// by path and every map field in `Note` is ordered (`BTreeMap` / sorted
/// `Vec`), so equal corpora always produce equal bytes and hashes.
pub fn build_snapshot(mut notes: Vec<Note>) -> CorpusSnapshot {
    notes.sort_by(|a, b| a.path.cmp(&b.path));
    let raw = serde_json::to_vec(&notes).unwrap_or_default();
    let hash = format!("{:016x}", fnv1a64(&raw));

    let mut br = Vec::with_capacity(raw.len() / 4);
    let params = brotli::enc::BrotliEncoderParams {
        quality: 11,
        ..Default::default()
    };
    if let Err(e) = brotli::BrotliCompress(&mut raw.as_slice(), &mut br, &params) {
        eprintln!("[corpus] brotli compression failed: {e}");
        br.clear();
    }

    let mut gz_encoder =
        flate2::write::GzEncoder::new(Vec::with_capacity(raw.len() / 3), flate2::Compression::best());
    let gz = gz_encoder
        .write_all(&raw)
        .and_then(|_| gz_encoder.finish())
        .unwrap_or_else(|e| {
            eprintln!("[corpus] gzip compression failed: {e}");
            Vec::new()
        });

    CorpusSnapshot {
        hash,
        raw: Bytes::from(raw),
        br: Bytes::from(br),
        gz: Bytes::from(gz),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthConfigFile, ConfigAuthzPolicy};
    use crate::cache::NotesCache;
    use crate::repository::DefaultRepository;
    use async_trait::async_trait;
    use notes_kit_core::error::StorageError;
    use notes_kit_core::models::{NoteId, NoteMetadata, SummaryFields};
    use notes_kit_core::traits::{AuthzPolicy, NoteFormat, NoteRepository, StorageBackend};
    use std::sync::{Arc, RwLock};

    struct StubStorage;

    #[async_trait]
    impl StorageBackend for StubStorage {
        async fn list_files(&self, _extension: &str) -> Result<Vec<String>, StorageError> {
            Ok(vec![
                "pub-hello.org".into(),
                "private-secret.org".into(),
                "unsigned-implicit.org".into(),
            ])
        }

        async fn read_file(&self, path: &str) -> Result<String, StorageError> {
            Ok(format!("body of {path} SECRET-MARKER-{path}"))
        }

        fn is_path_safe(&self, _path: &str) -> bool {
            true
        }
    }

    struct StubFormat;

    impl NoteFormat for StubFormat {
        fn extract_metadata(&self, _content: &str, filename: &str) -> NoteMetadata {
            NoteMetadata {
                id: None,
                title: Some(filename.to_string()),
                date: None,
                tags: Vec::new(),
                note_type: None,
                // Like denote `==sig`: explicit on some notes, absent on
                // others (absent must default to private).
                signature: if filename.starts_with("pub-") {
                    Some("public".to_string())
                } else if filename.starts_with("private-") {
                    Some("private".to_string())
                } else {
                    None
                },
            }
        }

        fn parse_id(&self, _filename: &str) -> Option<NoteId> {
            None
        }

        fn file_extension(&self) -> &str {
            "org"
        }

        fn summary_fields(&self, _content: &str) -> SummaryFields {
            SummaryFields::default()
        }
    }

    /// THE privacy gate for the whole corpus-asset design: a snapshot
    /// built the only sanctioned way — `list_accessible(anonymous_grants)`
    /// under the production-default policy (default signature private,
    /// anonymous = {public}) — must never contain a private or unsigned
    /// note, in any field.
    #[tokio::test]
    async fn public_snapshot_never_contains_private_notes() {
        let authz: Arc<dyn AuthzPolicy> =
            Arc::new(ConfigAuthzPolicy::from_config(AuthConfigFile::default()));
        let repository = DefaultRepository::new(
            Arc::new(StubStorage),
            Arc::new(StubFormat),
            Arc::clone(&authz),
            Arc::new(RwLock::new(NotesCache::default())),
        );
        repository.init_cache().await.expect("cache init");

        let anon_notes = repository
            .list_accessible(&authz.anonymous_grants())
            .await
            .expect("list accessible");
        let snapshot = build_snapshot(anon_notes);
        let json = String::from_utf8(snapshot.raw.to_vec()).expect("utf8");

        assert!(json.contains("pub-hello.org"), "public note missing");
        assert!(
            !json.contains("private-secret.org")
                && !json.contains("SECRET-MARKER-private-secret.org"),
            "explicitly private note leaked into the public snapshot"
        );
        assert!(
            !json.contains("unsigned-implicit.org"),
            "unsigned note (default-private) leaked into the public snapshot"
        );
    }
}

/// Axum handler for `GET /data/corpus/{file}` where `file` is
/// `<hash>.json`. Immutable-cached: the hash names the exact bytes.
pub async fn serve_corpus(
    axum::extract::Path(file): axum::extract::Path<String>,
    axum::Extension(state): axum::Extension<crate::state::AppState>,
    headers: http::HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let Some(hash) = file.strip_suffix(".json") else {
        return (http::StatusCode::NOT_FOUND, "not found").into_response();
    };
    let Some(snapshot) = state.corpus_snapshots.get(hash) else {
        return (http::StatusCode::NOT_FOUND, "unknown corpus version").into_response();
    };

    let accept = headers
        .get(http::header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let (body, encoding) = if accept.contains("br") && !snapshot.br.is_empty() {
        (snapshot.br.clone(), Some("br"))
    } else if accept.contains("gzip") && !snapshot.gz.is_empty() {
        (snapshot.gz.clone(), Some("gzip"))
    } else {
        (snapshot.raw.clone(), None)
    };

    let mut response = (
        [
            (http::header::CONTENT_TYPE, "application/json"),
            (
                http::header::CACHE_CONTROL,
                "public, max-age=31536000, immutable",
            ),
            (http::header::VARY, "Accept-Encoding"),
        ],
        body,
    )
        .into_response();
    if let Some(encoding) = encoding {
        response.headers_mut().insert(
            http::header::CONTENT_ENCODING,
            http::HeaderValue::from_static(encoding),
        );
    }
    response
}
