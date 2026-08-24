//! Blotter binary entrypoint — serves the dashboard and book API.
use finstack_blotter::{run, AppStateConfig};
use std::net::SocketAddr;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let addr: SocketAddr = std::env::var("BLOTTER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
        .parse()
        .expect("BLOTTER_ADDR must be host:port");

    let storage_path = std::env::var("BLOTTER_STORAGE_PATH").ok().map(Into::into);
    let demo_path = std::env::var("BLOTTER_DEMO_PATH").ok().map(Into::into);
    let ingest_token = std::env::var("BOOK_INGEST_TOKEN").ok();
    let config = AppStateConfig {
        storage_path,
        demo_path,
        ingest_token,
    };

    run(addr, config).await
}
