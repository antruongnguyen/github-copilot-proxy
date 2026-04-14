# Features

## OpenAI-Compatible API

Drop-in replacement for any tool that speaks the OpenAI protocol.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/chat/completions` | POST | Chat completions (streaming + non-streaming) |
| `/v1/embeddings` | POST | Text embeddings |
| `/v1/models` | GET | List available models (fetched live from Copilot) |

Standard OpenAI request/response format. Tools like aider, Continue, and Codex CLI work without modification.

## Dynamic Model Discovery

The `/v1/models` endpoint fetches the actual list of models available to your GitHub Copilot account. No hardcoded list — if GitHub adds new models to your plan, they appear automatically.

Models typically include GPT-5.4, GPT-5.3-codex, Claude Sonnet, Gemini, embedding models, and more.

## Streaming

Server-Sent Events (SSE) streaming is fully supported. When `"stream": true` is set in the request, the proxy passes through chunks from Copilot byte-for-byte with no buffering. This enables real-time token-by-token output in tools that support it.

## Web-Based Authentication

On first run (or when tokens expire), the proxy:
1. Requests a device code from GitHub
2. Opens your browser to a page showing the code and a button to authorize
3. Polls in the background until you complete authorization
4. Caches tokens for future runs

The browser only opens when authentication is actually needed — not on every startup.

### Auth Status Page (`/`)

The root page shows:
- **Initializing** — device flow starting
- **Pending** — device code displayed with copy button + GitHub link
- **Authenticated** — ready status with full model list
- **Error** — what went wrong

The page auto-polls `/auth/status` and updates in real-time when auth completes.

## Account Switching

Click "Sign out & switch account" on the web UI, or call `POST /auth/logout`. This:
- Clears in-memory tokens
- Deletes cached token files
- Starts a new device flow

No proxy restart required.

## Token Auto-Refresh

Copilot API tokens expire periodically. The proxy checks token expiration before each request and silently refreshes using the cached GitHub access token. If that fails (e.g., GitHub token revoked), it falls back to a new device flow.

## Optional API Key Protection

Start with `--api-key <key>` to require clients to send `Authorization: Bearer <key>`. Useful when running on a shared machine or exposing the proxy on a network.

The `/health`, `/`, and `/auth/*` endpoints bypass API key checks.

## Configuration

All settings via CLI flags or environment variables:

| Setting | Flag | Env | Default |
|---------|------|-----|---------|
| Host | `--host` | `COPILOT_PROXY_HOST` | `0.0.0.0` |
| Port | `--port` | `COPILOT_PROXY_PORT` | `6789` |
| API Key | `--api-key` | `COPILOT_PROXY_API_KEY` | none |
| Log Level | `--log-level` | `COPILOT_PROXY_LOG_LEVEL` | `info` |
| Token Dir | `--token-dir` | `COPILOT_PROXY_TOKEN_DIR` | `~/.config/copilot-proxy/` |

## Health Check

`GET /health` returns `{"status": "ok"}` — useful for monitoring and load balancer probes.

## Graceful Shutdown

The proxy shuts down cleanly on SIGINT (Ctrl+C) or SIGTERM, finishing in-flight requests before exiting.
