use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use uuid::Uuid;

const COPILOT_CHAT_VERSION: &str = "0.26.7";
const API_VERSION: &str = "2025-04-01";

pub fn build_copilot_headers(copilot_token: &str) -> HeaderMap {
    base_headers(copilot_token)
}

/// Build headers for /responses endpoint, including X-Initiator derived from the request body.
pub fn build_copilot_response_headers(copilot_token: &str, body: &str) -> HeaderMap {
    let mut headers = base_headers(copilot_token);
    let initiator = detect_initiator(body);
    headers.insert("X-Initiator", HeaderValue::from_static(initiator));
    headers
}

fn base_headers(copilot_token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {copilot_token}")).unwrap(),
    );
    headers.insert(
        "editor-version",
        HeaderValue::from_static("vscode/1.95.0"),
    );
    headers.insert(
        "editor-plugin-version",
        HeaderValue::from_str(&format!("copilot-chat/{COPILOT_CHAT_VERSION}")).unwrap(),
    );
    headers.insert(
        "User-Agent",
        HeaderValue::from_str(&format!("GitHubCopilotChat/{COPILOT_CHAT_VERSION}")).unwrap(),
    );
    headers.insert(
        "openai-intent",
        HeaderValue::from_static("conversation-panel"),
    );
    headers.insert(
        "x-github-api-version",
        HeaderValue::from_static(API_VERSION),
    );
    headers.insert(
        "x-request-id",
        HeaderValue::from_str(&Uuid::new_v4().to_string()).unwrap(),
    );
    headers.insert(
        "copilot-integration-id",
        HeaderValue::from_static("vscode-chat"),
    );
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    headers
}

/// Detect X-Initiator value from request body.
/// Returns "agent" if input contains assistant messages or items without a role.
/// Returns "user" otherwise.
fn detect_initiator(body: &str) -> &'static str {
    let Ok(parsed) = serde_json::from_str::<Value>(body) else {
        return "user";
    };

    let Some(input) = parsed.get("input") else {
        return "user";
    };

    let Some(items) = input.as_array() else {
        return "user";
    };

    for item in items {
        let Some(obj) = item.as_object() else {
            continue;
        };
        match obj.get("role").and_then(|r| r.as_str()) {
            None => return "agent",
            Some(role) if role.eq_ignore_ascii_case("assistant") => return "agent",
            _ => {}
        }
    }

    "user"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_contain_required_fields() {
        let headers = build_copilot_headers("test-token");

        assert_eq!(
            headers.get("Authorization").unwrap().to_str().unwrap(),
            "Bearer test-token"
        );
        assert_eq!(
            headers.get("editor-version").unwrap().to_str().unwrap(),
            "vscode/1.95.0"
        );
        assert!(headers
            .get("editor-plugin-version")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("copilot-chat/"));
        assert!(headers
            .get("User-Agent")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("GitHubCopilotChat/"));
        assert_eq!(
            headers.get("openai-intent").unwrap().to_str().unwrap(),
            "conversation-panel"
        );
        assert_eq!(
            headers.get("x-github-api-version").unwrap().to_str().unwrap(),
            "2025-04-01"
        );
        assert!(headers.get("x-request-id").is_some());
        assert_eq!(
            headers.get("copilot-integration-id").unwrap().to_str().unwrap(),
            "vscode-chat"
        );
        assert_eq!(
            headers.get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );
    }

    #[test]
    fn x_request_id_is_unique() {
        let h1 = build_copilot_headers("t");
        let h2 = build_copilot_headers("t");
        assert_ne!(
            h1.get("x-request-id").unwrap(),
            h2.get("x-request-id").unwrap()
        );
    }

    #[test]
    fn response_headers_include_x_initiator() {
        let headers = build_copilot_response_headers("t", r#"{"input":"hello"}"#);
        assert_eq!(
            headers.get("X-Initiator").unwrap().to_str().unwrap(),
            "user"
        );
        // Should also have all base headers
        assert!(headers.get("Authorization").is_some());
        assert!(headers.get("x-request-id").is_some());
    }

    #[test]
    fn detect_initiator_user_for_string_input() {
        assert_eq!(detect_initiator(r#"{"input":"hello"}"#), "user");
    }

    #[test]
    fn detect_initiator_user_for_user_messages() {
        let body = r#"{"input":[{"role":"user","content":"hi"}]}"#;
        assert_eq!(detect_initiator(body), "user");
    }

    #[test]
    fn detect_initiator_agent_for_assistant_messages() {
        let body = r#"{"input":[{"role":"user","content":"hi"},{"role":"assistant","content":"hey"}]}"#;
        assert_eq!(detect_initiator(body), "agent");
    }

    #[test]
    fn detect_initiator_agent_for_missing_role() {
        let body = r#"{"input":[{"content":"hi"}]}"#;
        assert_eq!(detect_initiator(body), "agent");
    }

    #[test]
    fn detect_initiator_user_for_invalid_json() {
        assert_eq!(detect_initiator("not json"), "user");
    }

    #[test]
    fn detect_initiator_user_for_no_input() {
        assert_eq!(detect_initiator(r#"{"model":"gpt-5.4"}"#), "user");
    }
}
