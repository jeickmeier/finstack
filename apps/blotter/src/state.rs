use crate::types::Book;
use anyhow::{Context, Result};
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::warn;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Current live book (persisted on update)
    pub book: Arc<RwLock<Book>>,
    /// Broadcast channel for SSE clients (sends full book snapshots)
    pub tx: broadcast::Sender<Book>,
    /// Storage path of the current book
    pub storage_path: PathBuf,
    /// Demo book payload (served separately; never mixed into live book)
    pub demo_book: Book,
    /// Optional ingest token to authorize POST /api/book
    pub ingest_token: Option<String>,
}

impl AppState {
    /// Initialize application state, creating the default book file if missing.
    pub fn initialize(config: AppStateConfig) -> Result<Self> {
        let storage_path = config
            .storage_path
            .unwrap_or_else(|| PathBuf::from("apps/blotter/data/book.json"));
        let storage_dir = storage_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("apps/blotter/data"));
        fs::create_dir_all(&storage_dir)
            .with_context(|| format!("creating storage dir {}", storage_dir.display()))?;

        let book = if Path::new(&storage_path).exists() {
            let data = fs::read_to_string(&storage_path)
                .with_context(|| format!("reading {}", storage_path.display()))?;
            serde_json::from_str::<Book>(&data)
                .with_context(|| format!("parsing {}", storage_path.display()))?
        } else {
            let b = Book::default();
            let data = serde_json::to_string_pretty(&b)?;
            fs::write(&storage_path, data)
                .with_context(|| format!("writing {}", storage_path.display()))?;
            b
        };

        let demo_book = if let Some(p) = &config.demo_path {
            let data =
                fs::read_to_string(p).with_context(|| format!("reading demo {}", p.display()))?;
            serde_json::from_str::<Book>(&data).with_context(|| "parsing demo json")?
        } else {
            // If no explicit demo fixture is provided, fallback to the live default book.
            Book::default()
        };

        let (tx, _rx) = broadcast::channel(64);

        Ok(Self {
            book: Arc::new(RwLock::new(book)),
            tx,
            storage_path,
            demo_book,
            ingest_token: config
                .ingest_token
                .or_else(|| std::env::var("BOOK_INGEST_TOKEN").ok()),
        })
    }
}

impl FromRef<AppState> for broadcast::Sender<Book> {
    fn from_ref(state: &AppState) -> Self {
        state.tx.clone()
    }
}

/// Configuration inputs for application state initialization.
#[derive(Clone, Default)]
pub struct AppStateConfig {
    /// Path to persist the live book JSON (defaults to apps/blotter/data/book.json)
    pub storage_path: Option<PathBuf>,
    /// Optional demo fixture file path
    pub demo_path: Option<PathBuf>,
    /// Optional ingest token (overrides env)
    pub ingest_token: Option<String>,
}

/// Convenience error wrapper for API responses.
pub struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let msg = self.0.to_string();
        let body = Json(json!({ "error": msg }));
        (StatusCode::BAD_REQUEST, body).into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        let e = err.into();
        warn!("API error: {e:#}");
        ApiError(e)
    }
}
