use std::path::PathBuf;

use clap::Parser;

fn default_token_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("copilot-proxy")
}

#[derive(Parser, Debug, Clone)]
#[command(name = "copilot-proxy", about = "OpenAI-compatible proxy for GitHub Copilot")]
pub struct Config {
    /// Host address to bind to
    #[arg(long, env = "COPILOT_PROXY_HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// Port to listen on
    #[arg(long, env = "COPILOT_PROXY_PORT", default_value_t = 6789)]
    pub port: u16,

    /// Optional API key for local client authentication
    #[arg(long, env = "COPILOT_PROXY_API_KEY")]
    pub api_key: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "COPILOT_PROXY_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    /// Directory for token storage
    #[arg(long, env = "COPILOT_PROXY_TOKEN_DIR", default_value_os_t = default_token_dir())]
    pub token_dir: PathBuf,
}

impl Config {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_addr_formats_correctly() {
        let config = Config {
            host: "0.0.0.0".into(),
            port: 9090,
            api_key: None,
            log_level: "info".into(),
            token_dir: PathBuf::from("/tmp"),
        };
        assert_eq!(config.bind_addr(), "0.0.0.0:9090");
    }

    #[test]
    fn bind_addr_default_values() {
        let config = Config {
            host: "0.0.0.0".into(),
            port: 6789,
            api_key: None,
            log_level: "info".into(),
            token_dir: PathBuf::from("/tmp"),
        };
        assert_eq!(config.bind_addr(), "0.0.0.0:6789");
    }
}
