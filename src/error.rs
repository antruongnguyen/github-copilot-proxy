use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    Auth(String),
    Proxy(String),
    Internal(String),
}

#[derive(Serialize)]
struct OpenAiError {
    error: OpenAiErrorBody,
}

#[derive(Serialize)]
struct OpenAiErrorBody {
    message: String,
    r#type: String,
    code: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            AppError::Auth(msg) => (StatusCode::UNAUTHORIZED, "authentication_error", msg.clone()),
            AppError::Proxy(msg) => (StatusCode::BAD_GATEWAY, "proxy_error", msg.clone()),
            AppError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg.clone())
            }
        };

        let body = OpenAiError {
            error: OpenAiErrorBody {
                message,
                r#type: error_type.to_string(),
                code: Some(status.as_u16().to_string()),
            },
        };

        (status, axum::Json(body)).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Auth(msg) => write!(f, "auth error: {msg}"),
            AppError::Proxy(msg) => write!(f, "proxy error: {msg}"),
            AppError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use http_body_util::BodyExt;

    use super::*;

    #[tokio::test]
    async fn auth_error_returns_401() {
        let err = AppError::Auth("bad token".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["type"], "authentication_error");
        assert_eq!(json["error"]["message"], "bad token");
    }

    #[tokio::test]
    async fn proxy_error_returns_502() {
        let err = AppError::Proxy("upstream down".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["type"], "proxy_error");
    }

    #[tokio::test]
    async fn internal_error_returns_500() {
        let err = AppError::Internal("oops".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["type"], "internal_error");
    }

    #[test]
    fn display_formats_correctly() {
        assert_eq!(
            AppError::Auth("x".into()).to_string(),
            "auth error: x"
        );
        assert_eq!(
            AppError::Proxy("y".into()).to_string(),
            "proxy error: y"
        );
        assert_eq!(
            AppError::Internal("z".into()).to_string(),
            "internal error: z"
        );
    }
}
