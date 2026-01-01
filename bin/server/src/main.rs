#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{Router, routing::get};
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use silver_telegram_authz::AuthzClient;
    use silver_telegram_server::{
        app::App,
        auth::{
            self, AppState, GmailOAuthClient, GmailOAuthState, OidcClient, db::SessionRepository,
            gmail_callback, gmail_start,
        },
        config::ServerConfig,
    };
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;
    use tower_http::services::ServeDir;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from environment
    let config = ServerConfig::from_env().expect("failed to load configuration");
    tracing::info!("Loaded configuration");

    // Create database connection pool
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("failed to connect to database");

    // Run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("failed to run migrations");

    // Cleanup expired sessions on startup
    let session_repo = SessionRepository::new(db_pool.clone());
    match session_repo.delete_expired().await {
        Ok(count) if count > 0 => {
            tracing::info!(
                deleted_sessions = count,
                "Cleaned up expired sessions on startup"
            );
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "Failed to cleanup expired sessions on startup");
        }
    }

    // Spawn periodic session cleanup task
    let cleanup_pool = db_pool.clone();
    let cleanup_interval_secs = config.session.cleanup_interval_seconds;
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(cleanup_interval_secs));
        loop {
            interval.tick().await;
            let repo = SessionRepository::new(cleanup_pool.clone());
            match repo.delete_expired().await {
                Ok(count) if count > 0 => {
                    tracing::debug!(deleted_sessions = count, "Periodic session cleanup");
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to cleanup expired sessions");
                }
            }
        }
    });

    // Initialize OIDC client
    tracing::info!("Discovering OIDC provider...");
    let oidc_client = OidcClient::discover(config.oidc)
        .await
        .expect("failed to discover OIDC provider");

    // Initialize SpiceDB authorization client
    tracing::info!("Connecting to SpiceDB...");
    let authz_client = AuthzClient::new(config.spicedb.endpoint, config.spicedb.preshared_key)
        .await
        .expect("failed to connect to SpiceDB");

    // Load the authorization schema
    const AUTHZ_SCHEMA: &str = include_str!("../../../lib/authz/schema.zed");
    tracing::info!("Loading SpiceDB authorization schema...");
    authz_client
        .write_schema(AUTHZ_SCHEMA)
        .await
        .expect("failed to write SpiceDB schema");

    let authz_client = Arc::new(authz_client);

    // Initialize Gmail OAuth client if configured
    let gmail_oauth_state = if config.google.is_configured() {
        tracing::info!("Gmail OAuth is configured");
        match GmailOAuthClient::new(&config.google) {
            Ok(client) => Some(GmailOAuthState {
                oauth_client: client,
                db_pool: db_pool.clone(),
                secure_cookies: config.session.secure_cookies,
            }),
            Err(e) => {
                tracing::warn!("Failed to initialize Gmail OAuth client: {}", e);
                None
            }
        }
    } else {
        tracing::info!(
            "Gmail OAuth not configured (set GOOGLE__CLIENT_ID, GOOGLE__CLIENT_SECRET, GOOGLE__REDIRECT_URL)"
        );
        None
    };

    // Create application state
    let app_state = Arc::new(AppState::new(db_pool, oidc_client, config.session.clone()));

    let conf = get_configuration(None).expect("failed to get leptos configuration");
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // Create combined state for Leptos routes
    let combined_state = CombinedState {
        leptos_options: leptos_options.clone(),
        app_state: app_state.clone(),
    };

    // Clone resources for use in the context layer
    let db_pool_for_context = app_state.db_pool.clone();
    let oidc_config_for_context = app_state.oidc_client.config().clone();
    let authz_client_for_context = authz_client.clone();

    // Build Gmail OAuth sub-router if configured
    let gmail_router: Option<Router<()>> = gmail_oauth_state.map(|gmail_state| {
        Router::new()
            .route("/auth/gmail/start", get(gmail_start))
            .route("/auth/gmail/callback", get(gmail_callback))
            .with_state(gmail_state)
    });

    // Build OIDC auth routes as a sub-router
    let oidc_router: Router<()> = Router::new()
        .route("/auth/login", get(auth::login))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/logout", get(auth::logout))
        .with_state(app_state.clone());

    // Build main router - start with Leptos routes that need CombinedState
    let mut app = Router::new()
        .leptos_routes(&combined_state, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<CombinedState, _>(
            shell,
        ))
        .with_state(combined_state);

    // Merge OIDC auth routes
    app = app.merge(oidc_router);

    // Merge Gmail routes if configured
    if let Some(gmail_routes) = gmail_router {
        app = app.merge(gmail_routes);
    }

    // Add layers and static file serving
    let app = app
        .nest_service("/pkg", ServeDir::new("target/site/pkg"))
        // Provide database pool, OIDC config, and authz client as request extensions
        .layer(axum::Extension(db_pool_for_context))
        .layer(axum::Extension(oidc_config_for_context))
        .layer(axum::Extension(authz_client_for_context));

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind to address");

    tracing::info!("listening on http://{}", addr);

    axum::serve(listener, app.into_make_service())
        .await
        .expect("server error");
}

/// Combined state for the application.
#[cfg(feature = "ssr")]
#[derive(Clone)]
struct CombinedState {
    leptos_options: leptos::prelude::LeptosOptions,
    app_state: std::sync::Arc<silver_telegram_server::auth::AppState>,
}

#[cfg(feature = "ssr")]
impl axum::extract::FromRef<CombinedState> for leptos::prelude::LeptosOptions {
    fn from_ref(state: &CombinedState) -> Self {
        state.leptos_options.clone()
    }
}

#[cfg(feature = "ssr")]
impl axum::extract::FromRef<CombinedState>
    for std::sync::Arc<silver_telegram_server::auth::AppState>
{
    fn from_ref(state: &CombinedState) -> Self {
        state.app_state.clone()
    }
}

#[cfg(feature = "ssr")]
fn shell(options: leptos::prelude::LeptosOptions) -> impl leptos::prelude::IntoView {
    use leptos::prelude::*;
    use leptos_meta::*;
    use silver_telegram_server::app::App;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="stylesheet" href="/pkg/silver-telegram.css"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // This main function is only used for WASM builds
    // The actual hydration happens in lib.rs
}
