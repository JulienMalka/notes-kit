use std::sync::{Arc, RwLock};

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

    let app_state = AppState {
        repository,
        auth_backend: auth_backend.clone(),
        authz_policy: authz,
        site_config: config.site.clone(),
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
        .service(tower_http::services::ServeDir::new(
            std::path::Path::new(&*site_root).join(&*pkg_dir),
        ));

    // Other static assets: cache for 1 day, revalidate after.
    let static_cache = tower_http::set_header::SetResponseHeaderLayer::if_not_present(
        http::header::CACHE_CONTROL,
        http::HeaderValue::from_static("public, max-age=86400, stale-while-revalidate=604800"),
    );
    let static_service = tower::ServiceBuilder::new()
        .layer(static_cache)
        .service(tower_http::services::ServeDir::new(&*site_root));

    let app = Router::new()
        .route("/api/events/notes", axum::routing::get(sse_notes))
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
        .layer(axum::Extension(version_rx))
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
