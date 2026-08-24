//! Integration tests for the blotter app (API and defaults).
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use finstack_blotter::{build_router, AppStateConfig};
use http_body_util::BodyExt;
use hyper::Method;
use hyper::Response;
use std::fs;
use std::path::PathBuf;
use tower::util::ServiceExt; // for `oneshot` // for collect()

fn tmp_path(file: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("finstack-blotter-tests");
    let _ = fs::create_dir_all(&dir);
    dir.join(file)
}

async fn oneshot(app: axum::Router, req: Request<Body>) -> Response<Body> {
    app.oneshot(req).await.unwrap()
}

#[tokio::test]
async fn get_book_defaults_to_flat_real_book() {
    let storage = tmp_path("book_default.json");
    let cfg = AppStateConfig {
        storage_path: Some(storage.clone()),
        demo_path: None,
        ingest_token: None,
    };
    let (app, _state) = build_router(cfg);

    let res = oneshot(
        app.clone(),
        Request::builder()
            .method(Method::GET)
            .uri("/api/book")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let book: finstack_blotter::types::Book = serde_json::from_slice(&body).unwrap();
    let default_book = finstack_blotter::types::Book::default();
    assert_eq!(
        serde_json::to_value(book).unwrap(),
        serde_json::to_value(default_book).unwrap()
    );
    // ensure file persisted
    assert!(storage.exists());
}

#[tokio::test]
async fn ingest_requires_bearer_token_and_persists() {
    let storage = tmp_path("book_ingest.json");
    let token = "secret-token";
    let cfg = AppStateConfig {
        storage_path: Some(storage.clone()),
        demo_path: None,
        ingest_token: Some(token.to_string()),
    };
    let (app, _state) = build_router(cfg);

    // Missing Authorization
    let res = oneshot(
        app.clone(),
        Request::builder()
            .method(Method::POST)
            .uri("/api/book")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"mode":"paper","live_enabled":false,"as_of":"2026-08-24T22:15:43Z","risk":{"max_inventory_shares_per_token":50,"max_quote_size":10,"max_open_markets":3,"max_notional_usd":200,"stale_after_seconds":900,"kill_on":[]},"kill_switch":{"armed":true,"tripped":false,"reason":null,"tripped_at":null},"quotes":[],"inventory":[],"fills":[],"pnl":{"realized_usd":0.0,"unrealized_usd":0.0},"last_pricer_sheet":null}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Wrong token
    let res = oneshot(
        app.clone(),
        Request::builder()
            .method(Method::POST)
            .uri("/api/book")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer wrong")
            .body(Body::from(r#"{"mode":"paper","live_enabled":false,"as_of":"2026-08-24T22:15:43Z","risk":{"max_inventory_shares_per_token":50,"max_quote_size":10,"max_open_markets":3,"max_notional_usd":200,"stale_after_seconds":900,"kill_on":[]},"kill_switch":{"armed":true,"tripped":false,"reason":null,"tripped_at":null},"quotes":[],"inventory":[],"fills":[],"pnl":{"realized_usd":0.0,"unrealized_usd":0.0},"last_pricer_sheet":null}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // Correct token
    let res = oneshot(
        app.clone(),
        Request::builder()
            .method(Method::POST)
            .uri("/api/book")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(r#"{"mode":"paper","live_enabled":false,"as_of":"2026-08-24T22:15:43Z","risk":{"max_inventory_shares_per_token":50,"max_quote_size":10,"max_open_markets":3,"max_notional_usd":200,"stale_after_seconds":900,"kill_on":[]},"kill_switch":{"armed":true,"tripped":false,"reason":null,"tripped_at":null},"quotes":[],"inventory":[],"fills":[],"pnl":{"realized_usd":0.0,"unrealized_usd":0.0},"last_pricer_sheet":null}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    // persisted
    assert!(storage.exists());
    let raw = fs::read_to_string(&storage).unwrap();
    assert!(raw.contains("\"mode\": \"paper\""));
}
