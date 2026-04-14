use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::Value;

use crate::auth::AuthManager;
use crate::error::AppError;
use crate::proxy::headers::{build_copilot_headers, build_copilot_response_headers};

#[derive(Clone)]
pub struct ProxyState {
    pub auth: Arc<AuthManager>,
    pub client: Client,
}

pub async fn chat_completions(
    State(state): State<ProxyState>,
    body: String,
) -> Result<Response, AppError> {
    forward_request(&state, "/chat/completions", &body).await
}

pub async fn embeddings(
    State(state): State<ProxyState>,
    body: String,
) -> Result<Response, AppError> {
    forward_request(&state, "/embeddings", &body).await
}

pub async fn responses(
    State(state): State<ProxyState>,
    body: String,
) -> Result<Response, AppError> {
    forward_response_request(&state, "/responses", &body).await
}

pub async fn responses_compact(
    State(state): State<ProxyState>,
    body: String,
) -> Result<Response, AppError> {
    forward_response_request(&state, "/responses", &body).await
}

async fn forward_request(state: &ProxyState, path: &str, body: &str) -> Result<Response, AppError> {
    let copilot_key = state.auth.get_copilot_token().await?;
    let headers = build_copilot_headers(&copilot_key.token);
    let url = format!("{}{path}", copilot_key.api_base());

    tracing::info!(url = %url, "forwarding request");

    let resp = state
        .client
        .post(&url)
        .headers(headers)
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| AppError::Proxy(format!("upstream request failed: {e}")))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

    if !resp.status().is_success() {
        let error_body = resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, body = %error_body, "upstream error");
        return Ok((status, error_body).into_response());
    }

    // Check if this is a streaming response
    let is_streaming = body
        .parse::<Value>()
        .ok()
        .and_then(|v| v.get("stream")?.as_bool())
        .unwrap_or(false);

    if is_streaming {
        let stream = resp
            .bytes_stream()
            .map(|chunk: Result<axum::body::Bytes, reqwest::Error>| {
                chunk.map_err(|e| std::io::Error::other(e.to_string()))
            });

        let body = Body::from_stream(stream);

        Ok(Response::builder()
            .status(status)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(body)
            .unwrap())
    } else {
        let response_body = resp
            .text()
            .await
            .map_err(|e| AppError::Proxy(format!("failed to read response: {e}")))?;

        Ok((
            status,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            response_body,
        )
            .into_response())
    }
}

/// Forward a request to the Copilot /responses endpoint with X-Initiator header.
/// The Responses API streams by default (SSE), so we always pass through as a stream
/// unless the response Content-Type indicates otherwise.
async fn forward_response_request(
    state: &ProxyState,
    path: &str,
    body: &str,
) -> Result<Response, AppError> {
    let copilot_key = state.auth.get_copilot_token().await?;
    let headers = build_copilot_response_headers(&copilot_key.token, body);
    let url = format!("{}{path}", copilot_key.api_base());

    tracing::info!(url = %url, "forwarding responses request");

    let resp = state
        .client
        .post(&url)
        .headers(headers)
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| AppError::Proxy(format!("upstream request failed: {e}")))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

    if !resp.status().is_success() {
        let error_body = resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, body = %error_body, "upstream error");
        return Ok((status, error_body).into_response());
    }

    // Detect streaming from the upstream response Content-Type
    let is_sse = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.contains("text/event-stream"));

    if is_sse {
        let stream = resp
            .bytes_stream()
            .map(|chunk: Result<axum::body::Bytes, reqwest::Error>| {
                chunk.map_err(|e| std::io::Error::other(e.to_string()))
            });

        Ok(Response::builder()
            .status(status)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(Body::from_stream(stream))
            .unwrap())
    } else {
        let response_body = resp
            .text()
            .await
            .map_err(|e| AppError::Proxy(format!("failed to read response: {e}")))?;

        Ok((
            status,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            response_body,
        )
            .into_response())
    }
}
