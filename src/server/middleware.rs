use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Clone)]
pub struct ApiKeyState {
    pub api_key: Option<String>,
}

pub async fn api_key_middleware(
    State(state): State<ApiKeyState>,
    req: Request,
    next: Next,
) -> Response {
    let Some(expected_key) = &state.api_key else {
        return next.run(req).await;
    };

    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth_header {
        Some(key) if key == expected_key => next.run(req).await,
        _ => {
            let body = json!({
                "error": {
                    "message": "Invalid API key",
                    "type": "authentication_error",
                    "code": "401"
                }
            });
            (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
        }
    }
}
