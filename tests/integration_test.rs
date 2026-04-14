use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use copilot_proxy::auth::AuthManager;
use copilot_proxy::server::router::create_router;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn test_auth_manager() -> Arc<AuthManager> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let _ = dir.keep();
    Arc::new(AuthManager::new(path))
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = create_router(test_auth_manager(), None);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn api_key_middleware_rejects_missing_key() {
    let app = create_router(test_auth_manager(), Some("secret".into()));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["type"], "authentication_error");
}

#[tokio::test]
async fn api_key_middleware_rejects_wrong_key() {
    let app = create_router(test_auth_manager(), Some("secret".into()));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .header("Authorization", "Bearer wrong")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_key_middleware_rejects_on_chat_completions() {
    let app = create_router(test_auth_manager(), Some("secret".into()));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"model":"gpt-5.4","messages":[]}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn health_bypasses_api_key() {
    let app = create_router(test_auth_manager(), Some("secret".into()));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = create_router(test_auth_manager(), None);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn root_page_returns_html() {
    let app = create_router(test_auth_manager(), None);

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("Copilot Proxy"));
    assert!(html.contains("/auth/status"));
}

#[tokio::test]
async fn auth_status_returns_json() {
    let app = create_router(test_auth_manager(), None);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/auth/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Fresh AuthManager with no tokens → status is "initializing"
    assert!(json["status"].is_string());
}

#[tokio::test]
async fn root_and_auth_status_bypass_api_key() {
    let app = create_router(test_auth_manager(), Some("secret".into()));

    // Root page — no key needed
    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn api_key_middleware_rejects_on_responses() {
    let app = create_router(test_auth_manager(), Some("secret".into()));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"model":"gpt-5.4","input":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_key_middleware_rejects_on_responses_compact() {
    let app = create_router(test_auth_manager(), Some("secret".into()));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses/compact")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"model":"gpt-5.4","input":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
