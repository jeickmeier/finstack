use crate::state::{ApiError, AppState};
use crate::types::Book;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures_util::Stream;
use futures_util::StreamExt as _;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::services::ServeDir;

/// GET /api/book — return the current live book.
pub async fn get_book(State(state): State<AppState>) -> Result<Json<Book>, ApiError> {
    let book = state.book.read().await.clone();
    Ok(Json(book))
}

/// GET /api/demo — return the demo book (never mixed into live book).
pub async fn get_demo(State(state): State<AppState>) -> Result<Json<Book>, ApiError> {
    Ok(Json(state.demo_book))
}

/// POST /api/book — replace the current live book (authorized).
pub async fn post_book(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(new_book): Json<Book>,
) -> Result<impl IntoResponse, ApiError> {
    // Auth: Authorization: Bearer <token>
    let Some(expected) = &state.ingest_token else {
        return Err(ApiError::from(anyhow::anyhow!(
            "ingest disabled: BOOK_INGEST_TOKEN not configured"
        )));
    };
    let Some(auth) = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    else {
        return Ok((StatusCode::UNAUTHORIZED, "missing Authorization").into_response());
    };
    let prefix = "Bearer ";
    let Some(token) = auth.strip_prefix(prefix) else {
        return Ok((StatusCode::UNAUTHORIZED, "invalid Authorization scheme").into_response());
    };
    if token != expected {
        return Ok((StatusCode::FORBIDDEN, "invalid token").into_response());
    }

    // Persist and broadcast
    {
        let mut guard = state.book.write().await;
        *guard = new_book.clone();
        let json = serde_json::to_string_pretty(&*guard)?;
        tokio::fs::write(&state.storage_path, json).await?;
    }
    let _ = state.tx.send(new_book);
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// GET /api/stream — server-sent events of book snapshots.
pub async fn sse_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| async move {
        match msg {
            Ok(book) => serde_json::to_string(&book)
                .ok()
                .map(|raw| Ok(Event::default().event("book").data(raw))),
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Static file service for the dashboard.
pub fn static_service() -> ServeDir {
    ServeDir::new("apps/blotter/static").append_index_html_on_directories(true)
}
