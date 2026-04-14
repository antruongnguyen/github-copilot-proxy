pub mod copilot_token;
pub mod device_flow;
pub mod storage;

use std::path::PathBuf;
use std::sync::Arc;

use reqwest::Client;
use tokio::sync::RwLock;

use crate::error::AppError;
use device_flow::AuthStatus;
use storage::CopilotApiKey;

pub struct AuthManager {
    client: Client,
    token_dir: PathBuf,
    github_token: Arc<RwLock<Option<String>>>,
    copilot_key: Arc<RwLock<Option<CopilotApiKey>>>,
    auth_status: Arc<RwLock<AuthStatus>>,
}

impl AuthManager {
    pub fn new(token_dir: PathBuf) -> Self {
        let github_token = storage::load_access_token(&token_dir);
        let copilot_key = storage::load_api_key(&token_dir).filter(|k| !k.is_expired());

        let initial_status = if copilot_key.is_some() {
            AuthStatus::Authenticated
        } else {
            AuthStatus::Initializing
        };

        Self {
            client: Client::new(),
            token_dir,
            github_token: Arc::new(RwLock::new(github_token)),
            copilot_key: Arc::new(RwLock::new(copilot_key)),
            auth_status: Arc::new(RwLock::new(initial_status)),
        }
    }

    /// Get the current auth status for the web UI.
    pub async fn get_auth_status(&self) -> AuthStatus {
        self.auth_status.read().await.clone()
    }

    /// Get a valid Copilot API token, refreshing or authenticating as needed.
    pub async fn get_copilot_token(&self) -> Result<CopilotApiKey, AppError> {
        // Fast path: cached and valid
        {
            let key = self.copilot_key.read().await;
            if let Some(k) = key.as_ref() {
                if !k.is_expired() {
                    return Ok(k.clone());
                }
            }
        }

        // Slow path: need to refresh
        let mut key_guard = self.copilot_key.write().await;

        // Double-check after acquiring write lock
        if let Some(k) = key_guard.as_ref() {
            if !k.is_expired() {
                return Ok(k.clone());
            }
        }

        // Need a GitHub access token first
        let gh_token = self.ensure_github_token().await?;

        let api_key =
            match copilot_token::fetch_copilot_token(&self.client, &gh_token).await {
                Ok(key) => key,
                Err(e) => {
                    // Token might be invalid, clear it so next attempt re-authenticates
                    let mut tok_guard = self.github_token.write().await;
                    *tok_guard = None;
                    return Err(e);
                }
            };
        storage::save_api_key(&self.token_dir, &api_key)?;

        *self.auth_status.write().await = AuthStatus::Authenticated;
        *key_guard = Some(api_key.clone());
        Ok(api_key)
    }

    /// Try to authenticate on startup. Returns true if device flow was needed (browser should open).
    pub async fn startup_auth(&self) -> bool {
        // Already have a valid copilot token?
        {
            let key = self.copilot_key.read().await;
            if let Some(k) = key.as_ref() {
                if !k.is_expired() {
                    *self.auth_status.write().await = AuthStatus::Authenticated;
                    tracing::info!("Copilot token loaded from cache");
                    return false;
                }
            }
        }

        // Have a github token? Try to get copilot token from it.
        {
            let gh_tok = self.github_token.read().await;
            if let Some(token) = gh_tok.as_ref() {
                match copilot_token::fetch_copilot_token(&self.client, token).await {
                    Ok(api_key) => {
                        let _ = storage::save_api_key(&self.token_dir, &api_key);
                        *self.copilot_key.write().await = Some(api_key);
                        *self.auth_status.write().await = AuthStatus::Authenticated;
                        tracing::info!("Copilot token refreshed from cached GitHub token");
                        return false;
                    }
                    Err(e) => {
                        tracing::warn!("Cached GitHub token failed: {e}, starting device flow");
                    }
                }
            }
        }

        // Need device flow
        self.run_device_flow_with_status().await;
        true
    }

    /// Clear all cached tokens and restart device flow.
    pub async fn logout_and_reauth(&self) {
        // Clear in-memory state
        *self.github_token.write().await = None;
        *self.copilot_key.write().await = None;

        // Clear persisted tokens
        let _ = std::fs::remove_file(storage::access_token_path(&self.token_dir));
        let _ = std::fs::remove_file(storage::api_key_path(&self.token_dir));

        tracing::info!("Logged out, starting new device flow");
        self.run_device_flow_with_status().await;
    }

    async fn run_device_flow_with_status(&self) {
        *self.auth_status.write().await = AuthStatus::Initializing;

        let device_resp = match device_flow::initiate_device_flow(&self.client).await {
            Ok(resp) => resp,
            Err(e) => {
                *self.auth_status.write().await = AuthStatus::Error {
                    message: e.to_string(),
                };
                return;
            }
        };

        tracing::info!(
            user_code = %device_resp.user_code,
            "Device flow started — waiting for user authorization"
        );

        *self.auth_status.write().await = AuthStatus::Pending {
            user_code: device_resp.user_code.clone(),
            verification_uri: device_resp.verification_uri.clone(),
        };

        let interval = device_resp.interval.unwrap_or(5);
        match device_flow::poll_for_token(&self.client, &device_resp.device_code, interval).await {
            Ok(token) => {
                let _ = storage::save_access_token(&self.token_dir, &token);
                *self.github_token.write().await = Some(token.clone());

                match copilot_token::fetch_copilot_token(&self.client, &token).await {
                    Ok(api_key) => {
                        let _ = storage::save_api_key(&self.token_dir, &api_key);
                        *self.copilot_key.write().await = Some(api_key);
                        *self.auth_status.write().await = AuthStatus::Authenticated;
                        tracing::info!("Authentication successful");
                    }
                    Err(e) => {
                        *self.auth_status.write().await = AuthStatus::Error {
                            message: format!("Got GitHub token but Copilot token failed: {e}"),
                        };
                    }
                }
            }
            Err(e) => {
                *self.auth_status.write().await = AuthStatus::Error {
                    message: e.to_string(),
                };
            }
        }
    }

    async fn ensure_github_token(&self) -> Result<String, AppError> {
        {
            let tok = self.github_token.read().await;
            if let Some(t) = tok.as_ref() {
                return Ok(t.clone());
            }
        }

        let mut tok_guard = self.github_token.write().await;
        if let Some(t) = tok_guard.as_ref() {
            return Ok(t.clone());
        }

        tracing::info!("No GitHub access token found, starting device flow...");
        let token = device_flow::run_device_flow(&self.client).await?;
        storage::save_access_token(&self.token_dir, &token)?;

        *tok_guard = Some(token.clone());
        Ok(token)
    }
}
