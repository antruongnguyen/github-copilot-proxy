use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const POLL_INTERVAL_SECS: u64 = 5;
const MAX_POLL_ATTEMPTS: u32 = 60; // 5 minutes

#[derive(Deserialize, Clone, Debug)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[allow(dead_code)]
    pub expires_in: u64,
    pub interval: Option<u64>,
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

/// Auth flow status visible to the web UI.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status")]
pub enum AuthStatus {
    /// Already authenticated, no action needed.
    #[serde(rename = "authenticated")]
    Authenticated,
    /// Waiting for user to enter the code at GitHub.
    #[serde(rename = "pending")]
    Pending {
        user_code: String,
        verification_uri: String,
    },
    /// Authentication in progress (initiating device flow).
    #[serde(rename = "initializing")]
    Initializing,
    /// Authentication failed.
    #[serde(rename = "error")]
    Error { message: String },
}

/// Step 1: Request a device code from GitHub.
pub async fn initiate_device_flow(client: &Client) -> Result<DeviceCodeResponse, AppError> {
    let resp: DeviceCodeResponse = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", GITHUB_CLIENT_ID), ("scope", "copilot")])
        .send()
        .await
        .map_err(|e| AppError::Auth(format!("device code request failed: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Auth(format!("device code parse failed: {e}")))?;

    Ok(resp)
}

/// Step 2: Poll GitHub until user authorizes or timeout.
pub async fn poll_for_token(
    client: &Client,
    device_code: &str,
    interval: u64,
) -> Result<String, AppError> {
    let interval = if interval == 0 {
        POLL_INTERVAL_SECS
    } else {
        interval
    };

    for _ in 0..MAX_POLL_ATTEMPTS {
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

        let token_resp: AccessTokenResponse = client
            .post(ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", GITHUB_CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|e| AppError::Auth(format!("token poll failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::Auth(format!("token poll parse failed: {e}")))?;

        if let Some(token) = token_resp.access_token {
            return Ok(token);
        }

        match token_resp.error.as_deref() {
            Some("authorization_pending") | Some("slow_down") => continue,
            Some(err) => return Err(AppError::Auth(format!("device flow error: {err}"))),
            None => continue,
        }
    }

    Err(AppError::Auth("device flow timed out".into()))
}

/// All-in-one device flow (kept for backward compatibility).
pub async fn run_device_flow(client: &Client) -> Result<String, AppError> {
    let resp = initiate_device_flow(client).await?;

    eprintln!();
    eprintln!("  Please visit: {}", resp.verification_uri);
    eprintln!("  And enter code: {}", resp.user_code);
    eprintln!();

    let interval = resp.interval.unwrap_or(POLL_INTERVAL_SECS);
    poll_for_token(client, &resp.device_code, interval).await
}
