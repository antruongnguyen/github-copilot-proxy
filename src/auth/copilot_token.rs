use reqwest::Client;

use crate::auth::storage::CopilotApiKey;
use crate::error::AppError;

const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";

pub async fn fetch_copilot_token(
    client: &Client,
    github_access_token: &str,
) -> Result<CopilotApiKey, AppError> {
    let resp = client
        .get(COPILOT_TOKEN_URL)
        .header(
            "Authorization",
            format!("token {github_access_token}"),
        )
        .header("Accept", "application/json")
        .header("User-Agent", "copilot-proxy")
        .send()
        .await
        .map_err(|e| AppError::Auth(format!("copilot token request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Auth(format!(
            "copilot token request returned {status}: {body}"
        )));
    }

    resp.json::<CopilotApiKey>()
        .await
        .map_err(|e| AppError::Auth(format!("copilot token parse failed: {e}")))
}
