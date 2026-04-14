# GitHub Copilot Proxy — Implementation Plan

## Overview

A Rust HTTP proxy that exposes an OpenAI-compatible API (`/v1/chat/completions`, `/v1/responses`, `/v1/embeddings`, `/v1/models`) backed by GitHub Copilot's API (`api.githubcopilot.com`). Users authenticate once via GitHub's OAuth Device Flow, and the proxy handles token lifecycle, request transformation, and response streaming transparently.

## Architecture

```
┌─────────────┐      ┌──────────────────┐      ┌──────────────────────────┐
│  Client     │      │  copilot-proxy   │      │  api.githubcopilot.com   │
│  (codex,    │─────>│                  │─────>│                          │
│   aider,    │      │  :6789           │      │  /chat/completions       │
│   curl)     │<─────│                  │<─────│  /responses              │
└─────────────┘      └──────────────────┘      └──────────────────────────┘
                            │
                            ▼
                     GitHub OAuth
                     Device Flow
```

### Request Flow

1. Client sends OpenAI-format request to `http://localhost:6789/v1/chat/completions`
2. Proxy validates local API key (optional, configurable)
3. Proxy obtains/refreshes Copilot API token (cached, auto-refreshed)
4. Proxy rewrites request headers (add Copilot auth + required headers)
5. Proxy forwards to `https://api.githubcopilot.com/chat/completions`
6. Proxy streams response back to client

## Technical Decisions

### Language & Framework
- **Rust** with `tokio` async runtime
- **axum** for HTTP server
- **reqwest** for HTTP client with streaming support (10-minute timeout for reasoning models)

### Authentication: GitHub OAuth Device Flow
- Two-phase token acquisition:
  1. **GitHub access token** via device flow (one-time, user authorizes in browser)
  2. **Copilot API token** via `api.github.com/copilot_internal/v2/token` (auto-refreshed)
- Tokens cached to `~/.config/copilot-proxy/`
- Web UI at `/` for browser-based device code display
- Account switching via `POST /auth/logout`

### API Compatibility
- `/v1/chat/completions` — chat completions (streaming + non-streaming)
- `/v1/responses` — OpenAI Responses API (Codex CLI `wire_api = "responses"`)
- `/v1/responses/compact` — context compaction for long Codex sessions
- `/v1/embeddings` — text embeddings
- `/v1/models` — dynamic model list from Copilot API
- 16MB request body limit for large Codex contexts

### Copilot API Headers
```
Authorization: Bearer {copilot_token}
editor-version: vscode/1.95.0
editor-plugin-version: copilot-chat/0.26.7
user-agent: GitHubCopilotChat/0.26.7
openai-intent: conversation-panel
x-github-api-version: 2025-04-01
x-request-id: {uuid}
copilot-integration-id: vscode-chat
X-Initiator: user|agent  (for /responses endpoint)
```

## Module Structure

```
src/
├── main.rs              # CLI parsing, server startup, background auth
├── config.rs            # Configuration (CLI args + env vars)
├── error.rs             # Unified error types → OpenAI-format JSON
├── auth/
│   ├── mod.rs           # AuthManager: token lifecycle, startup auth, logout
│   ├── device_flow.rs   # GitHub OAuth Device Flow (initiate + poll)
│   ├── copilot_token.rs # Copilot API token acquisition + refresh
│   └── storage.rs       # Token file persistence
├── proxy/
│   ├── mod.rs
│   ├── handler.rs       # Request forwarding (chat, responses, embeddings, streaming)
│   ├── headers.rs       # Copilot-specific headers + X-Initiator
│   └── models.rs        # /v1/models (dynamic + static fallback)
└── server/
    ├── mod.rs
    ├── router.rs         # axum routes, web UI, auth endpoints, body limit
    └── middleware.rs     # Optional API key validation
```

## Configuration

| Setting | Env Var | CLI Flag | Default |
|---------|---------|----------|---------|
| Listen address | `COPILOT_PROXY_HOST` | `--host` | `0.0.0.0` |
| Listen port | `COPILOT_PROXY_PORT` | `--port` | `6789` |
| Local API key | `COPILOT_PROXY_API_KEY` | `--api-key` | none |
| Log level | `COPILOT_PROXY_LOG_LEVEL` | `--log-level` | `info` |
| Token directory | `COPILOT_PROXY_TOKEN_DIR` | `--token-dir` | `~/.config/copilot-proxy/` |

## Implementation Phases

### Phase 1-5: Core Implementation (completed)
- Project skeleton, CLI, axum server, health endpoint
- GitHub device flow auth with token caching and auto-refresh
- `/v1/chat/completions` proxy with SSE streaming
- `/v1/embeddings` and `/v1/models` endpoints
- API key middleware, graceful shutdown

### Phase 6: Web UI (completed)
- Browser-based auth page with device code display
- Auto-polling auth status with real-time UI updates
- Dynamic model listing after authentication
- Account switching (sign out + re-auth)
- Browser auto-open only when auth needed

### Phase 7: Codex CLI Support (completed)
- `/v1/responses` endpoint with X-Initiator header
- `/v1/responses/compact` endpoint for context compaction
- 16MB request body limit for large Codex contexts
- 10-minute request timeout for reasoning models
- Codex CLI configuration example in README

### Phase 8: Tests (completed)
- 38 unit and integration tests
- Run with `cargo test`

### Phase 9: Open Source Release (completed)
- MIT license (An Nguyen), CHANGELOG, .editorconfig
- README with install, usage, tool integration examples
- docs/overview.md, docs/features.md, docs/developer-guide.md
- Default port 6789, default host 0.0.0.0

## Status

v1.0.0 — All phases completed. The proxy is functional, tested, and ready for open-source release.
