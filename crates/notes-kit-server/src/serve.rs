use std::sync::{Arc, RwLock};

use crate::asset_repository::AssetRepository;
use crate::auth::{
    AuthConfigFile, ConfigAuthzPolicy, DynAuthnBackend, SqliteAuthBackend, UserRepository,
};
use crate::cache::NotesCache;
use crate::config::ServerConfig;
use crate::repository::DefaultRepository;
use crate::state::AppState;
use notes_kit_core::traits::{AuthzPolicy, StorageBackend};

#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("config error: {0}")]
    Config(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("server error: {0}")]
    Server(String),
}

pub async fn serve<AppFn, AppView, ShellFn, ShellView>(
    config: ServerConfig,
    storage: Arc<dyn StorageBackend>,
    asset_storage: Option<Arc<dyn StorageBackend>>,
    format: Arc<dyn notes_kit_core::traits::NoteFormat>,
    app_fn: AppFn,
    shell_fn: ShellFn,
) -> Result<(), ServeError>
where
    AppFn: Fn() -> AppView + Clone + Send + Sync + 'static,
    AppView: leptos::prelude::IntoView + 'static,
    ShellFn: Fn(leptos::prelude::LeptosOptions) -> ShellView + Clone + Send + Sync + 'static,
    ShellView: leptos::prelude::IntoView + 'static,
{
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use sqlx::SqlitePool;
    use tower_sessions::cookie::SameSite;
    use tower_sessions::SessionManagerLayer;
    use tower_sessions_sqlx_store::SqliteStore;

    let auth_config = if let Some(ref path) = config.auth_config {
        let content =
            std::fs::read_to_string(path).map_err(|e| ServeError::Auth(e.to_string()))?;
        toml::from_str::<AuthConfigFile>(&content)
            .map_err(|e| ServeError::Auth(e.to_string()))?
    } else {
        AuthConfigFile::default()
    };

    let admin_config = auth_config.admin.clone();
    let authz: Arc<dyn AuthzPolicy> = Arc::new(ConfigAuthzPolicy::from_config(auth_config));

    let db_url = format!("sqlite:{}?mode=rwc", config.user_db_path);
    let user_repo = UserRepository::new(&db_url)
        .await
        .map_err(|e| ServeError::Auth(e.to_string()))?;
    user_repo
        .migrate()
        .await
        .map_err(|e| ServeError::Auth(e.to_string()))?;

    if let Some(ref admin) = admin_config {
        user_repo
            .ensure_admin(admin)
            .await
            .map_err(|e| ServeError::Auth(e.to_string()))?;
    }

    let auth_backend: Arc<dyn notes_kit_core::traits::AuthBackend> =
        Arc::new(SqliteAuthBackend::new(user_repo));

    let cache = Arc::new(RwLock::new(NotesCache::default()));
    let repository = Arc::new(DefaultRepository::new(
        storage,
        format.clone(),
        authz.clone(),
        cache,
    ));

    repository
        .init_cache()
        .await
        .map_err(|e| ServeError::Config(e.to_string()))?;

    let initial_hash = repository.global_version_hash();
    let (version_tx, version_rx) = tokio::sync::watch::channel(initial_hash);

    {
        let repo = Arc::clone(&repository);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            let mut last_listing_hash: Option<u64> = None;
            let mut last_content_hash = initial_hash;
            interval.tick().await;
            loop {
                interval.tick().await;
                match repo.listing_hash().await {
                    Ok(Some(h)) if Some(h) == last_listing_hash => continue,
                    Ok(Some(h)) => {
                        eprintln!("[cache] listing changed, reloading notes");
                        last_listing_hash = Some(h);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("[cache] listing hash error: {e}");
                        continue;
                    }
                }
                let old_count = repo.cached_note_count();
                if let Err(e) = repo.refresh_cache().await {
                    eprintln!("[cache] refresh error: {e}");
                    continue;
                }
                let new_count = repo.cached_note_count();
                let hash = repo.global_version_hash();
                if hash != last_content_hash {
                    eprintln!(
                        "[cache] change detected: hash {last_content_hash:#x} -> {hash:#x}, notes {old_count} -> {new_count}"
                    );
                    last_content_hash = hash;
                    let _ = version_tx.send(hash);
                }
            }
        });
    }

    let asset_repository = if let Some(asset_store) = asset_storage {
        let repo = Arc::new(AssetRepository::new(asset_store, authz.clone()));
        repo.init_cache()
            .await
            .map_err(|e| ServeError::Config(format!("asset cache init: {e}")))?;

        {
            let repo = Arc::clone(&repo);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
                let mut last_listing_hash: Option<u64> = None;
                interval.tick().await;
                loop {
                    interval.tick().await;
                    match repo.listing_hash().await {
                        Ok(h) if Some(h) == last_listing_hash => continue,
                        Ok(h) => {
                            eprintln!("[cache] asset listing changed, reloading assets");
                            last_listing_hash = Some(h);
                        }
                        Err(e) => {
                            eprintln!("[cache] asset listing hash error: {e}");
                            continue;
                        }
                    }
                    let old_count = repo.cached_asset_count();
                    if let Err(e) = repo.refresh_cache().await {
                        eprintln!("[cache] asset refresh error: {e}");
                        continue;
                    }
                    let new_count = repo.cached_asset_count();
                    eprintln!("[cache] assets reloaded: {old_count} -> {new_count}");
                }
            });
        }

        Some(repo)
    } else {
        None
    };

    let app_state = AppState {
        repository,
        auth_backend: auth_backend.clone(),
        authz_policy: authz,
        site_config: config.site.clone(),
        asset_repository,
    };

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(app_fn);

    let session_pool = SqlitePool::connect("sqlite:.sessions.db?mode=rwc")
        .await
        .map_err(|e| ServeError::Server(e.to_string()))?;
    let session_store = SqliteStore::new(session_pool);
    session_store
        .migrate()
        .await
        .map_err(|e| ServeError::Server(e.to_string()))?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_same_site(SameSite::Lax)
        .with_expiry(tower_sessions::Expiry::OnInactivity(
            tower_sessions::cookie::time::Duration::days(7),
        ));

    let axum_auth_backend = DynAuthnBackend(auth_backend);
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(axum_auth_backend, session_layer)
        .build();

    let site_root = leptos_options.site_root.clone();
    let pkg_dir = leptos_options.site_pkg_dir.clone();
    let shell = {
        let options = leptos_options.clone();
        move || shell_fn(options.clone())
    };

    // Content-hashed assets in /pkg/ can be cached indefinitely.
    let pkg_cache = tower_http::set_header::SetResponseHeaderLayer::if_not_present(
        http::header::CACHE_CONTROL,
        http::HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    let pkg_service = tower::ServiceBuilder::new()
        .layer(pkg_cache)
        .service(
            tower_http::services::ServeDir::new(
                std::path::Path::new(&*site_root).join(&*pkg_dir),
            )
            .precompressed_br()
            .precompressed_gzip(),
        );

    // Other static assets: cache for 1 day, revalidate after.
    let static_cache = tower_http::set_header::SetResponseHeaderLayer::if_not_present(
        http::header::CACHE_CONTROL,
        http::HeaderValue::from_static("public, max-age=86400, stale-while-revalidate=604800"),
    );
    let static_service = tower::ServiceBuilder::new()
        .layer(static_cache)
        .service(
            tower_http::services::ServeDir::new(&*site_root)
                .precompressed_br()
                .precompressed_gzip(),
        );

    let mut app = Router::new()
        .route("/api/events/notes", axum::routing::get(sse_notes));

    if app_state.asset_repository.is_some() {
        app = app.route("/assets/{*path}", axum::routing::get(serve_asset));
    }

    let sitemap_cfg = config.sitemap_base_url.clone().map(|base_url| SitemapConfig {
        base_url,
        static_paths: config.sitemap_static_paths.clone(),
    });
    if sitemap_cfg.is_some() {
        app = app.route("/sitemap.xml", axum::routing::get(serve_sitemap));
    }

    let app = app
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let state = app_state.clone();
                move || {
                    leptos::prelude::provide_context(state.clone());
                    leptos::prelude::provide_context(state.site_config.clone());
                }
            },
            shell,
        )
        .nest_service(&format!("/{pkg_dir}"), pkg_service)
        .fallback_service(static_service)
        .layer(axum::Extension(app_state))
        .layer(axum::Extension(version_rx))
        .layer(axum::Extension(sitemap_cfg))
        .layer(auth_layer)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| ServeError::Server(e.to_string()))?;
    eprintln!("[server] Listening on http://{addr}");
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| ServeError::Server(e.to_string()))?;

    Ok(())
}

async fn serve_asset(
    axum::extract::Path(path): axum::extract::Path<String>,
    auth_session: crate::auth::AuthSession,
    axum::Extension(state): axum::Extension<AppState>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let Some(ref asset_repo) = state.asset_repository else {
        return (http::StatusCode::NOT_FOUND, "assets not configured").into_response();
    };

    let grants = if let Some(ref user) = auth_session.user {
        state
            .authz_policy
            .grants_for_levels(&user.0.assigned_levels)
    } else {
        state.authz_policy.anonymous_grants()
    };

    let filename = path.rsplit('/').next().unwrap_or(&path);
    let signature = notes_kit_org::denote::DenoteFilename::parse(filename)
        .and_then(|d| d.signature);

    if !asset_repo.can_access_asset(signature.as_deref(), &grants) {
        return (http::StatusCode::FORBIDDEN, "access denied").into_response();
    }

    match asset_repo.read_bytes(&path).await {
        Ok(bytes) => {
            let content_type = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            let headers = [
                (http::header::CONTENT_TYPE, content_type),
                (
                    http::header::CACHE_CONTROL,
                    "public, max-age=86400, stale-while-revalidate=604800".to_string(),
                ),
            ];
            (headers, bytes).into_response()
        }
        Err(notes_kit_core::error::StorageError::NotFound(_)) => {
            (http::StatusCode::NOT_FOUND, "asset not found").into_response()
        }
        Err(e) => {
            (http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

#[derive(Clone)]
struct SitemapConfig {
    base_url: String,
    static_paths: Vec<String>,
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Extract the first valid YYYY-MM-DD substring from an arbitrary date string
/// (org-mode timestamps, partial years, etc). Returns None if no sensible date
/// is found so the sitemap omits `<lastmod>` rather than emitting spec-invalid values.
fn extract_iso_date(s: &str) -> Option<String> {
    if s.len() < 10 {
        return None;
    }
    let bytes = s.as_bytes();
    for i in 0..=bytes.len() - 10 {
        let slice = &s[i..i + 10];
        let sb = slice.as_bytes();
        if sb[4] != b'-' || sb[7] != b'-' {
            continue;
        }
        let digits_ok = sb.iter().enumerate().all(|(j, b)| {
            if j == 4 || j == 7 { true } else { b.is_ascii_digit() }
        });
        if !digits_ok {
            continue;
        }
        let year: i32 = slice[0..4].parse().unwrap_or(0);
        let month: u32 = slice[5..7].parse().unwrap_or(0);
        let day: u32 = slice[8..10].parse().unwrap_or(0);
        if year >= 1900 && (1..=12).contains(&month) && (1..=31).contains(&day) {
            return Some(slice.to_string());
        }
    }
    None
}

async fn serve_sitemap(
    auth_session: crate::auth::AuthSession,
    axum::Extension(state): axum::Extension<AppState>,
    axum::Extension(cfg): axum::Extension<Option<SitemapConfig>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let Some(cfg) = cfg else {
        return (http::StatusCode::NOT_FOUND, "sitemap not configured").into_response();
    };

    // Use anonymous grants for sitemap — only include publicly accessible notes.
    let _ = auth_session;
    let grants = state.authz_policy.anonymous_grants();

    let notes = match state.repository.list_accessible(&grants).await {
        Ok(n) => n,
        Err(e) => {
            return (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("sitemap error: {e}"),
            )
                .into_response();
        }
    };

    let base = cfg.base_url.trim_end_matches('/');

    let mut xml = String::with_capacity(4096);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);
    xml.push('\n');

    for path in &cfg.static_paths {
        let path = if path.starts_with('/') {
            path.clone()
        } else {
            format!("/{path}")
        };
        xml.push_str("  <url><loc>");
        xml.push_str(&xml_escape(&format!("{base}{path}")));
        xml.push_str("</loc></url>\n");
    }

    for note in &notes {
        if note.signature() != "public" {
            continue;
        }
        // Skip index notes — their content is served at the landing page (e.g. "/")
        // and including them creates duplicate URLs with the same content.
        if note.filename.contains("--index") {
            continue;
        }
        let url = format!("{base}/notes/{}", note.path);
        xml.push_str("  <url><loc>");
        xml.push_str(&xml_escape(&url));
        xml.push_str("</loc>");
        if let Some(date) = note.metadata.date.as_deref().and_then(extract_iso_date) {
            xml.push_str("<lastmod>");
            xml.push_str(&xml_escape(&date));
            xml.push_str("</lastmod>");
        }
        xml.push_str("</url>\n");
    }

    xml.push_str("</urlset>\n");

    let headers = [
        (http::header::CONTENT_TYPE, "application/xml; charset=utf-8".to_string()),
        (http::header::CACHE_CONTROL, "public, max-age=3600".to_string()),
    ];
    (headers, xml).into_response()
}

async fn sse_notes(
    axum::Extension(mut rx): axum::Extension<tokio::sync::watch::Receiver<u64>>,
) -> axum::response::sse::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>
{
    // Mark the current value as seen so the first changed() only fires on *new* updates,
    // not on values that were already present before this SSE connection was established.
    rx.borrow_and_update();
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        rx.changed().await.ok()?;
        Some((
            Ok(axum::response::sse::Event::default().data("changed")),
            rx,
        ))
    });
    axum::response::sse::Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
}
