use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::error::AppError;
use crate::proxy::handler::ProxyState;
use crate::proxy::headers::build_copilot_headers;

pub async fn list_models(State(state): State<ProxyState>) -> Result<Response, AppError> {
    let copilot_key = state.auth.get_copilot_token().await?;
    let headers = build_copilot_headers(&copilot_key.token);
    let url = format!("{}/models", copilot_key.api_base());

    tracing::info!(url = %url, "fetching models from copilot API");

    let resp = state
        .client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| AppError::Proxy(format!("models request failed: {e}")))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

    if !resp.status().is_success() {
        let error_body = resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, body = %error_body, "models endpoint error");
        return Ok((status, error_body).into_response());
    }

    let body = resp
        .text()
        .await
        .map_err(|e| AppError::Proxy(format!("failed to read models response: {e}")))?;

    Ok((
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body,
    )
        .into_response())
}

/// Fallback: return a static list when the upstream API is unreachable.
pub fn static_model_list() -> Value {
    use serde_json::json;

    const COPILOT_MODELS: &[&str] = &[
        "gpt-5.4",
        "gpt-5.4-mini",
        "gpt-5.3-codex",
        "gpt-5-mini",
        "gpt-4.1",
        "o3-mini",
        "o4-mini",
        "claude-opus-4.6",
        "claude-sonnet-4.6",
        "claude-haiku-4.5",
        "gemini-2.5-pro",
    ];

    let data: Vec<Value> = COPILOT_MODELS
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "object": "model",
                "created": 0,
                "owned_by": "github-copilot",
            })
        })
        .collect();

    json!({
        "object": "list",
        "data": data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_model_list_returns_valid_openai_format() {
        let resp = static_model_list();

        assert_eq!(resp["object"], "list");
        let data = resp["data"].as_array().unwrap();
        assert!(!data.is_empty());

        for model in data {
            assert!(model["id"].is_string());
            assert_eq!(model["object"], "model");
            assert_eq!(model["owned_by"], "github-copilot");
        }
    }

    #[test]
    fn static_model_list_includes_known_models() {
        let resp = static_model_list();
        let ids: Vec<&str> = resp["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap())
            .collect();

        assert!(ids.contains(&"gpt-5.4"));
        assert!(ids.contains(&"claude-sonnet-4.6"));
        assert!(ids.contains(&"gemini-2.5-pro"));
    }
}
