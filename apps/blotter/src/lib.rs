//! Finstack Blotter: lightweight Axum server and static dashboard for the
//! Polymarket weather market-making desk. This crate exposes the HTTP router
//! constructor so integration tests can start the app without binding a port.
//! All public items include documentation per workspace lint settings.

pub mod metrics;
mod routes;
mod state;
pub mod types;

use axum::{routing::get, Router};
use routes::{get_book, get_demo, post_book, sse_stream, static_service};
use state::AppState;
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

/// Build the Axum router with all routes and middleware.
///
/// # Arguments
/// - `config`: Application state configuration such as data paths and tokens.
///
/// Returns a tuple `(Router, AppState)` so callers with out-of-process
/// lifecycles (tests) can inspect or mutate state directly.
pub fn build_router(config: AppStateConfig) -> (Router, AppState) {
    let state = AppState::initialize(config).expect("failed to initialize app state");

    let router = Router::new()
        .route("/api/book", get(get_book).post(post_book))
        .route("/api/demo", get(get_demo))
        .route("/api/stream", get(sse_stream))
        .nest_service("/", static_service())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    (router, state)
}

/// Run the HTTP server, binding to the provided `addr`.
///
/// # Arguments
/// - `addr`: Socket address to bind, e.g. `127.0.0.1:8700`.
/// - `config`: Application state configuration (data dir, ingest token).
pub async fn run(addr: SocketAddr, config: AppStateConfig) -> anyhow::Result<()> {
    let (app, _state) = build_router(config);
    info!("Starting blotter on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

/// Re-export of configuration for external callers and tests.
pub use state::AppStateConfig;
