use std::sync::Arc;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::auth::AuthManager;
use crate::auth::device_flow::AuthStatus;
use crate::proxy::handler::{self, ProxyState};
use crate::proxy::models;
use crate::server::middleware::{ApiKeyState, api_key_middleware};

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn auth_status(State(state): State<Arc<AuthManager>>) -> Json<AuthStatus> {
    Json(state.get_auth_status().await)
}

async fn auth_logout(State(state): State<Arc<AuthManager>>) -> impl IntoResponse {
    tokio::spawn(async move {
        state.logout_and_reauth().await;
    });
    (StatusCode::OK, Json(json!({"status": "logging_out"})))
}

async fn root_page(State(state): State<Arc<AuthManager>>) -> Html<String> {
    let status = state.get_auth_status().await;
    Html(render_auth_page(&status))
}

fn render_auth_page(status: &AuthStatus) -> String {
    let (status_class, status_text, code_section) = match status {
        AuthStatus::Authenticated => (
            "authenticated",
            "Authenticated",
            String::from(
                r#"<p class="ready">GitHub Copilot proxy is ready.</p>
                <div id="models-list"><p class="loading">Loading models...</p></div>
                <button onclick="signOut()" class="sign-out-btn">Sign out &amp; switch account</button>"#,
            ),
        ),
        AuthStatus::Pending {
            user_code,
            verification_uri,
        } => (
            "pending",
            "Waiting for authorization",
            format!(
                r#"<div class="code-box">
                    <p>Enter this code at GitHub:</p>
                    <div class="user-code" id="user-code">{user_code}</div>
                    <button onclick="navigator.clipboard.writeText('{user_code}')" class="copy-btn">Copy code</button>
                    <a href="{verification_uri}" target="_blank" rel="noopener" class="btn">Open GitHub Device Login</a>
                </div>"#
            ),
        ),
        AuthStatus::Initializing => (
            "initializing",
            "Initializing",
            String::from(r#"<p>Starting authentication...</p>"#),
        ),
        AuthStatus::Error { message } => (
            "error",
            "Authentication failed",
            format!(r#"<p class="error-msg">{message}</p>"#),
        ),
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GitHub Copilot Proxy</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
         background: #0d1117; color: #e6edf3; display: flex; justify-content: center;
         align-items: center; min-height: 100vh; }}
  .card {{ background: #161b22; border: 1px solid #30363d; border-radius: 12px;
           padding: 2.5rem; max-width: 480px; width: 90%; text-align: center; }}
  h1 {{ font-size: 1.4rem; margin-bottom: 0.5rem; }}
  .status {{ display: inline-block; padding: 4px 12px; border-radius: 20px;
             font-size: 0.85rem; font-weight: 500; margin: 0.75rem 0 1.5rem; }}
  .status.authenticated {{ background: #1a7f37; color: #fff; }}
  .status.pending {{ background: #9e6a03; color: #fff; }}
  .status.initializing {{ background: #30363d; color: #8b949e; }}
  .status.error {{ background: #da3633; color: #fff; }}
  .code-box {{ margin-top: 1rem; }}
  .code-box p {{ color: #8b949e; margin-bottom: 0.75rem; }}
  .user-code {{ font-family: 'SF Mono', 'Fira Code', monospace; font-size: 2.2rem;
                font-weight: 700; letter-spacing: 0.15em; color: #58a6ff;
                background: #0d1117; border: 1px solid #30363d; border-radius: 8px;
                padding: 0.75rem 1.5rem; margin-bottom: 1rem; user-select: all; }}
  .btn, .copy-btn {{ display: inline-block; padding: 10px 20px; border-radius: 6px;
                     font-size: 0.95rem; font-weight: 500; text-decoration: none;
                     cursor: pointer; border: none; margin: 0.4rem; }}
  .btn {{ background: #238636; color: #fff; }}
  .btn:hover {{ background: #2ea043; }}
  .copy-btn {{ background: #30363d; color: #e6edf3; }}
  .copy-btn:hover {{ background: #484f58; }}
  .ready {{ color: #3fb950; margin-top: 0.5rem; margin-bottom: 1rem; }}
  .error-msg {{ color: #f85149; margin-top: 0.5rem; word-break: break-word; }}
  .loading {{ color: #8b949e; }}
  .models {{ text-align: left; margin: 1rem 0; }}
  .models h3 {{ font-size: 0.85rem; color: #8b949e; text-transform: uppercase;
                letter-spacing: 0.05em; margin-bottom: 0.5rem; text-align: center; }}
  .model-grid {{ display: flex; flex-wrap: wrap; gap: 6px; justify-content: center; }}
  .model-tag {{ background: #0d1117; border: 1px solid #30363d; border-radius: 6px;
                padding: 4px 10px; font-size: 0.8rem; font-family: 'SF Mono', monospace;
                color: #58a6ff; }}
  .sign-out-btn {{ background: none; border: 1px solid #30363d; color: #8b949e;
                   padding: 6px 14px; border-radius: 6px; font-size: 0.8rem;
                   cursor: pointer; margin-top: 1rem; }}
  .sign-out-btn:hover {{ border-color: #da3633; color: #f85149; }}
</style>
</head>
<body>
<div class="card">
  <h1>GitHub Copilot Proxy</h1>
  <span class="status {status_class}" id="status">{status_text}</span>
  <div id="content">{code_section}</div>
</div>
<script>
  async function loadModels() {{
    const el = document.getElementById('models-list');
    if (!el) return;
    try {{
      const r = await fetch('/v1/models');
      if (!r.ok) {{ el.innerHTML = '<p class="loading">Could not load models</p>'; return; }}
      const d = await r.json();
      const models = (d.data || []).map(m => m.id).sort();
      if (models.length === 0) {{ el.innerHTML = ''; return; }}
      el.innerHTML = '<div class="models"><h3>Available Models (' + models.length + ')</h3>'
        + '<div class="model-grid">' + models.map(m => '<span class="model-tag">' + m + '</span>').join('') + '</div></div>';
    }} catch(_) {{ el.innerHTML = ''; }}
  }}

  async function signOut() {{
    await fetch('/auth/logout', {{ method: 'POST' }});
    location.reload();
  }}

  const initial = '{status_class}';
  if (initial === 'authenticated') {{
    loadModels();
  }} else {{
    const poll = setInterval(async () => {{
      try {{
        const r = await fetch('/auth/status');
        const s = await r.json();
        if (s.status === 'authenticated') {{
          clearInterval(poll);
          document.getElementById('status').className = 'status authenticated';
          document.getElementById('status').textContent = 'Authenticated';
          document.getElementById('content').innerHTML =
            '<p class="ready">GitHub Copilot proxy is ready.</p>'
            + '<div id="models-list"><p class="loading">Loading models...</p></div>'
            + '<button onclick="signOut()" class="sign-out-btn">Sign out &amp; switch account</button>';
          loadModels();
        }}
      }} catch(_) {{}}
    }}, 2000);
  }}
</script>
</body>
</html>"##
    )
}

pub fn create_router(auth: Arc<AuthManager>, api_key: Option<String>) -> Router {
    let proxy_state = ProxyState {
        auth: auth.clone(),
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .expect("failed to build reqwest client"),
    };

    let api_key_state = ApiKeyState { api_key };

    let api_routes = Router::new()
        .route("/v1/chat/completions", post(handler::chat_completions))
        .route("/v1/embeddings", post(handler::embeddings))
        .route("/v1/responses", post(handler::responses))
        .route("/v1/responses/compact", post(handler::responses_compact))
        .route("/v1/models", get(models::list_models))
        .with_state(proxy_state)
        .route_layer(middleware::from_fn_with_state(
            api_key_state,
            api_key_middleware,
        ));

    Router::new()
        .route("/", get(root_page))
        .route("/health", get(health))
        .route("/auth/status", get(auth_status))
        .route("/auth/logout", post(auth_logout))
        .with_state(auth.clone())
        .merge(api_routes)
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024)) // 16MB for large Codex contexts
}
