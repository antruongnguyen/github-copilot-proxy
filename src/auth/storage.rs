use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CopilotApiKey {
    pub token: String,
    pub expires_at: i64,
    #[serde(default)]
    pub endpoints: CopilotEndpoints,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CopilotEndpoints {
    #[serde(default)]
    pub api: Option<String>,
}

impl CopilotApiKey {
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() >= self.expires_at
    }

    pub fn api_base(&self) -> &str {
        self.endpoints
            .api
            .as_deref()
            .unwrap_or("https://api.githubcopilot.com")
    }
}

pub fn token_dir(base: &Path) -> PathBuf {
    base.to_path_buf()
}

pub fn access_token_path(base: &Path) -> PathBuf {
    token_dir(base).join("access-token")
}

pub fn api_key_path(base: &Path) -> PathBuf {
    token_dir(base).join("api-key.json")
}

pub fn save_access_token(base: &Path, token: &str) -> Result<(), AppError> {
    let dir = token_dir(base);
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Internal(format!("failed to create token dir: {e}")))?;
    std::fs::write(access_token_path(base), token)
        .map_err(|e| AppError::Internal(format!("failed to write access token: {e}")))?;
    Ok(())
}

pub fn load_access_token(base: &Path) -> Option<String> {
    std::fs::read_to_string(access_token_path(base))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn save_api_key(base: &Path, key: &CopilotApiKey) -> Result<(), AppError> {
    let dir = token_dir(base);
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Internal(format!("failed to create token dir: {e}")))?;
    let json = serde_json::to_string_pretty(key)
        .map_err(|e| AppError::Internal(format!("failed to serialize api key: {e}")))?;
    std::fs::write(api_key_path(base), json)
        .map_err(|e| AppError::Internal(format!("failed to write api key: {e}")))?;
    Ok(())
}

pub fn load_api_key(base: &Path) -> Option<CopilotApiKey> {
    let data = std::fs::read_to_string(api_key_path(base)).ok()?;
    serde_json::from_str(&data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copilot_api_key_expired() {
        let key = CopilotApiKey {
            token: "tok".into(),
            expires_at: 0,
            endpoints: CopilotEndpoints::default(),
        };
        assert!(key.is_expired());
    }

    #[test]
    fn copilot_api_key_not_expired() {
        let key = CopilotApiKey {
            token: "tok".into(),
            expires_at: chrono::Utc::now().timestamp() + 3600,
            endpoints: CopilotEndpoints::default(),
        };
        assert!(!key.is_expired());
    }

    #[test]
    fn api_base_default() {
        let key = CopilotApiKey {
            token: "tok".into(),
            expires_at: 0,
            endpoints: CopilotEndpoints::default(),
        };
        assert_eq!(key.api_base(), "https://api.githubcopilot.com");
    }

    #[test]
    fn api_base_custom() {
        let key = CopilotApiKey {
            token: "tok".into(),
            expires_at: 0,
            endpoints: CopilotEndpoints {
                api: Some("https://custom.api.com".into()),
            },
        };
        assert_eq!(key.api_base(), "https://custom.api.com");
    }

    #[test]
    fn roundtrip_access_token() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        save_access_token(base, "my-token").unwrap();
        let loaded = load_access_token(base).unwrap();
        assert_eq!(loaded, "my-token");
    }

    #[test]
    fn load_access_token_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_access_token(dir.path()).is_none());
    }

    #[test]
    fn load_access_token_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(access_token_path(dir.path()), "").unwrap();
        assert!(load_access_token(dir.path()).is_none());
    }

    #[test]
    fn roundtrip_api_key() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        let key = CopilotApiKey {
            token: "copilot-tok".into(),
            expires_at: 1234567890,
            endpoints: CopilotEndpoints {
                api: Some("https://example.com".into()),
            },
        };

        save_api_key(base, &key).unwrap();
        let loaded = load_api_key(base).unwrap();
        assert_eq!(loaded.token, "copilot-tok");
        assert_eq!(loaded.expires_at, 1234567890);
        assert_eq!(loaded.api_base(), "https://example.com");
    }

    #[test]
    fn load_api_key_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_api_key(dir.path()).is_none());
    }

    #[test]
    fn api_key_deserializes_without_endpoints() {
        let json = r#"{"token":"t","expires_at":123}"#;
        let key: CopilotApiKey = serde_json::from_str(json).unwrap();
        assert_eq!(key.token, "t");
        assert_eq!(key.api_base(), "https://api.githubcopilot.com");
    }
}
