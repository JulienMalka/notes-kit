#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::sync::Arc;

    let notes_dir = std::env::var("NOTES_DIR").unwrap_or_else(|_| "./notes".to_string());

    let auth_config = std::env::var("AUTH_CONFIG").unwrap_or_else(|_| "./auth.toml".to_string());

    let mut config = notes_kit_server::config::ServerConfig::new(&notes_dir)
        .title("Minimal Notes")
        .port(3000);

    if std::path::Path::new(&auth_config).exists() {
        config = config.auth_config(auth_config);
    }

    let storage: Arc<dyn notes_kit_server::storage::StorageBackend> = Arc::new(
        notes_kit_server::storage::LocalStorageBackend::new(notes_dir.into())
            .expect("Failed to initialize local storage"),
    );

    let format = Arc::new(notes_kit_org::format::OrgFormat::default());

    if let Err(e) = notes_kit_server::serve::serve(
        config,
        storage,
        format,
        minimal_notes_app::App,
        shell,
    )
    .await
    {
        eprintln!("Server error: {e}");
        std::process::exit(1);
    }
}

#[cfg(feature = "ssr")]
fn shell(options: leptos::prelude::LeptosOptions) -> impl leptos::prelude::IntoView {
    use leptos::prelude::*;
    use leptos_meta::*;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
                <MetaTags />
                <link rel="stylesheet" href="/notes-kit.css" />
            </head>
            <body>
                <minimal_notes_app::App />
            </body>
        </html>
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {}
