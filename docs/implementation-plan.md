# GitHub Copilot Proxy — Implementation Plan

## Overview

A Rust HTTP proxy that exposes an OpenAI-compatible API (`/v1/chat/completions`, `/v1/embeddings`) backed by GitHub Copilot's API (`api.githubcopilot.com`). Users authenticate once via GitHub's OAuth Device Flow, and the proxy handles token lifecycle, request transformation, and response streaming transparently.

## Architecture

```
┌─────────────┐      ┌──────────────────┐      ┌──────────────────────────┐
│  Client     │      │  copilot-proxy   │      │  api.githubcopilot.com   │
│  (curl,     │─────▶│                  │─────▶│                          │
│   aider,    │      │  :8080           │      │  /chat/completions       │
│   continue) │◀─────│                  │◀─────│  /embeddings             │
└─────────────┘      └──────────────────┘      └──────────────────────────┘
                            │
                            ▼
                     GitHub OAuth
                     Device Flow
```

### Request Flow

1. Client sends OpenAI-format request to `http://localhost:8080/v1/chat/completions`
2. Proxy validates local API key (optional, configurable)
3. Proxy obtains/refreshes Copilot API token (cached, auto-refreshed)
4. Proxy rewrites request headers (add Copilot auth + required headers)
5. Proxy forwards to `https://api.githubcopilot.com/chat/completions`
6. Proxy streams response back to client

## Technical Decisions

### Language & Framework
- **Rust** with `tokio` async runtime
- **axum** for HTTP server (same as `hair` reference project — proven pattern)
- **reqwest** for HTTP client with streaming support

### Authentication: GitHub OAuth Device Flow
- Chosen over PKCE because Copilot uses GitHub's device flow natively
- Two-phase token acquisition:
  1. **GitHub access token** via device flow (one-time, user authorizes in browser)
  2. **Copilot API token** via `api.github.com/copilot_internal/v2/token` (auto-refreshed)
- Tokens cached to `~/.config/copilot-proxy/` (file-based, simple to debug)
- Background refresh before expiration

### API Compatibility
- OpenAI-compatible endpoints: `/v1/chat/completions`, `/v1/embeddings`, `/v1/models`
- Streaming via SSE (`text/event-stream`) — pass through from Copilot API
- Non-streaming responses also supported
- `/v1/models` returns available Copilot models

### Copilot API Headers (from LiteLLM reference)
```
Authorization: Bearer {copilot_token}
editor-version: vscode/1.95.0
editor-plugin-version: copilot-chat/0.26.7
user-agent: GitHubCopilotChat/0.26.7
openai-intent: conversation-panel
x-github-api-version: 2025-04-01
x-request-id: {uuid}
copilot-integration-id: vscode-chat
```

### Quota Protection
- The `ideas.md` requires avoiding premium quota drain
- Use non-premium models by default (configurable)
- Log model usage for visibility
- Optional rate limiting (requests/minute) as a safety net

### Local API Key (Optional)
- If configured, proxy validates `Authorization: Bearer {local_key}` from clients
- Prevents unauthorized local access when proxy is running

## Module Structure

```
src/
├── main.rs              # CLI parsing, server startup
├── config.rs            # Configuration (CLI args + env vars + config file)
├── server/
│   ├── mod.rs
│   └── router.rs        # axum routes: /v1/chat/completions, /v1/embeddings, /v1/models, /health
├── auth/
│   ├── mod.rs
│   ├── device_flow.rs   # GitHub OAuth Device Flow (step 1: get github access token)
│   ├── copilot_token.rs # Copilot API token acquisition + refresh (step 2)
│   └── storage.rs       # Token file persistence (~/.config/copilot-proxy/)
├── proxy/
│   ├── mod.rs
│   ├── handler.rs       # Request handler: validate → auth → rewrite → forward → stream
│   ├── headers.rs       # Copilot-specific header construction
│   └── models.rs        # /v1/models endpoint (list available models)
└── error.rs             # Unified error types
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `axum` | HTTP server |
| `reqwest` | HTTP client (streaming) |
| `serde` / `serde_json` | Serialization |
| `clap` | CLI argument parsing |
| `tracing` / `tracing-subscriber` | Structured logging |
| `uuid` | Request IDs |
| `chrono` | Token expiration handling |
| `dirs` | XDG config directory resolution |

## Configuration

Priority: CLI flags > environment variables > config file (`~/.config/copilot-proxy/config.toml`)

| Setting | Env Var | CLI Flag | Default |
|---------|---------|----------|---------|
| Listen address | `COPILOT_PROXY_HOST` | `--host` | `127.0.0.1` |
| Listen port | `COPILOT_PROXY_PORT` | `--port` | `8080` |
| Local API key | `COPILOT_PROXY_API_KEY` | `--api-key` | none (no auth) |
| Log level | `RUST_LOG` | `--log-level` | `info` |
| Token directory | `COPILOT_PROXY_TOKEN_DIR` | `--token-dir` | `~/.config/copilot-proxy/` |

## Implementation Phases

### Phase 1: Project Skeleton & Server
- Cargo.toml with dependencies
- CLI parsing with clap
- axum server with health endpoint
- Structured logging setup

### Phase 2: GitHub Authentication
- Device flow implementation (POST to github.com/login/device/code, poll for token)
- Copilot token acquisition (POST to api.github.com/copilot_internal/v2/token)
- File-based token caching with expiration check
- Auto-refresh on expired tokens

### Phase 3: Proxy — Chat Completions
- `/v1/chat/completions` route
- Request forwarding with Copilot headers
- SSE streaming pass-through
- Non-streaming response support
- Error mapping (Copilot errors → OpenAI-format errors)

### Phase 4: Proxy — Embeddings & Models
- `/v1/embeddings` route forwarding
- `/v1/models` route returning available Copilot models

### Phase 5: Configuration & Polish
- Optional local API key validation middleware
- Graceful shutdown

### Phase 6: Unit Tests (completed)
- 20 unit tests across config, error, auth/storage, proxy/headers, proxy/models
- 7 integration tests for router (health, API key middleware, models endpoint)
- Run with `cargo test`

## Status

All phases completed. The proxy is functional and tested.
