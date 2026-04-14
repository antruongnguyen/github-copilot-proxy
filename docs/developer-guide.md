# Developer Guide

## Prerequisites

- [Rust](https://rustup.rs/) 1.85+ (edition 2024)
- A GitHub account with Copilot access (for integration testing)

## Getting Started

```bash
git clone https://github.com/antruongnguyen/github-copilot-proxy.git
cd copilot-proxy
cargo build
cargo test
cargo run
```

## Project Structure

```
src/
├── main.rs              # Entry point: CLI parsing, server startup, background auth
├── lib.rs               # Module declarations
├── config.rs            # Configuration struct (clap + env vars)
├── error.rs             # Unified error types → OpenAI-format JSON responses
├── auth/
│   ├── mod.rs           # AuthManager: token lifecycle, startup auth, logout
│   ├── device_flow.rs   # GitHub OAuth Device Flow (initiate + poll)
│   ├── copilot_token.rs # Copilot API token acquisition from GitHub token
│   └── storage.rs       # File-based token persistence
├── proxy/
│   ├── mod.rs
│   ├── handler.rs       # Request forwarding (chat, embeddings, streaming)
│   ├── headers.rs       # Copilot-specific HTTP headers
│   └── models.rs        # /v1/models endpoint (dynamic + static fallback)
└── server/
    ├── mod.rs
    ├── router.rs         # axum routes, web UI HTML, auth endpoints
    └── middleware.rs     # Optional API key validation
tests/
└── integration_test.rs   # Router-level integration tests
```

## Architecture

### Request Flow

1. Client sends OpenAI-format request to proxy
2. `middleware.rs` — validates API key (if configured)
3. `handler.rs` — extracts request, calls `AuthManager.get_copilot_token()`
4. `AuthManager` — returns cached token or refreshes (double-checked locking)
5. `headers.rs` — builds Copilot-required headers
6. `handler.rs` — forwards to `api.githubcopilot.com`, streams response back

### Authentication Flow

```
                      ┌──────────┐
                      │  Cached  │──yes──▶ Return token
                      │  valid?  │
                      └────┬─────┘
                           │ no
                      ┌────▼─────┐
                      │  GitHub  │──yes──▶ Refresh Copilot token
                      │  token?  │
                      └────┬─────┘
                           │ no
                      ┌────▼─────┐
                      │  Device  │──▶ Show code in web UI
                      │  Flow    │──▶ Poll until authorized
                      └──────────┘
```

### Key Types

- `AuthManager` — thread-safe token manager using `Arc<RwLock<>>`. Handles caching, refresh, and device flow.
- `ProxyState` — axum state containing `AuthManager` + `reqwest::Client`. Shared across all handlers.
- `AppError` — error enum that implements `IntoResponse` for OpenAI-format JSON errors.
- `AuthStatus` — enum (`Authenticated | Pending | Initializing | Error`) serialized to JSON for the web UI.

## Testing

```bash
# Run all tests
cargo test

# Run a specific test
cargo test auth::storage::tests::roundtrip_access_token

# Run with output
cargo test -- --nocapture
```

### Test Categories

**Unit tests** (inline in modules):
- `config` — bind address formatting
- `error` — HTTP status codes, OpenAI JSON format, Display trait
- `auth/storage` — token expiry, API base URLs, file roundtrips, deserialization
- `proxy/headers` — required headers present, unique request IDs
- `proxy/models` — static fallback list format

**Integration tests** (`tests/integration_test.rs`):
- Health endpoint
- API key middleware (reject missing, reject wrong, accept correct)
- API key bypass for health/root/auth routes
- Root page HTML content
- Auth status JSON response
- 404 for unknown routes

Tests use `tower::ServiceExt::oneshot()` to test the full axum router without starting a server.

## Adding a New Endpoint

1. Add handler function in `src/proxy/handler.rs`:
   ```rust
   pub async fn my_endpoint(
       State(state): State<ProxyState>,
       body: String,
   ) -> Result<Response, AppError> {
       forward_request(&state, "/my-path", &body).await
   }
   ```

2. Wire the route in `src/server/router.rs`:
   ```rust
   .route("/v1/my-endpoint", post(handler::my_endpoint))
   ```

3. Add integration test in `tests/integration_test.rs`

4. Run `cargo test`

## Building for Release

```bash
cargo build --release
```

The binary is at `target/release/copilot-proxy`. Release builds use LTO and symbol stripping for a small binary.

## Environment Variables for Development

```bash
# Verbose logging
COPILOT_PROXY_LOG_LEVEL=debug cargo run

# Use a temp token directory (forces fresh auth)
cargo run -- --token-dir /tmp/copilot-test

# Run on a different port
cargo run -- --port 9999
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo test` and `cargo clippy`
5. Submit a pull request

Please keep changes focused and include tests for new functionality.
