# copilot-proxy v1.0.0

**Use GitHub Copilot models from any OpenAI-compatible tool.**

A lightweight Rust proxy that exposes GitHub Copilot's AI models through a standard OpenAI API. Authenticate once with your GitHub account, then use tools like [CLINE](https://cline.bot), [Claude Code](https://claude.com/product/claude-code), [Codex CLI](https://github.com/openai/codex), or plain `curl` against `http://localhost:6789/v1/`.

```
┌─────────────┐      ┌──────────────────┐      ┌──────────────────────────┐
│  Your tools │      │  copilot-proxy   │      │  api.githubcopilot.com   │
│  (aider,    │─────>│                  │─────>│                          │
│   codex,    │      │  :6789           │      │  /chat/completions       │
│   curl)     │<─────│                  │<─────│  /models                 │
└─────────────┘      └──────────────────┘      └──────────────────────────┘
```

## Features

- **OpenAI-compatible API** — drop-in replacement for any tool that speaks OpenAI
- **Dynamic model discovery** — lists all models available to your Copilot account
- **Streaming support** — real-time SSE streaming for chat completions
- **Web UI** — browser-based auth flow with device code display and model listing
- **Account switching** — sign out and re-authenticate without restarting
- **Auto token management** — tokens cached to disk, auto-refreshed on expiry
- **Optional API key** — protect the proxy with a local Bearer token
- **Codex CLI support** — `/v1/responses` and `/v1/responses/compact` endpoints
- **Fast** — built in Rust with async I/O, sub-millisecond proxy overhead

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) 1.85+
- A GitHub account with [Copilot](https://github.com/features/copilot) access

### Build and Run

```bash
git clone https://github.com/antruongnguyen/github-copilot-proxy.git
cd copilot-proxy
cargo build --release
./target/release/copilot-proxy
```

On first run, your browser opens with a device code. Enter it at GitHub to authorize. Subsequent runs use cached tokens.

### Test It

```bash
# List available models
curl http://localhost:6789/v1/models

# Chat completion
curl http://localhost:6789/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-5.4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Streaming
curl http://localhost:6789/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-5.4",
    "stream": true,
    "messages": [{"role": "user", "content": "Write a haiku about Rust"}]
  }'
```

## Use with Tools

### Codex CLI

```bash
# In ~/.codex/config.toml
model = "gpt-5.4"
model_provider = "copilot-proxy"

[model_providers.copilot-proxy]
name = "GitHub Copilot via copilot-proxy"
base_url = "http://localhost:6789/v1"
wire_api = "chat"
env_key = "COPILOT_PROXY_API_KEY"
```

Then: `codex --provider copilot-proxy "your prompt"`

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | Web UI — auth status, device code, model list |
| `GET` | `/health` | Health check |
| `GET` | `/auth/status` | Auth status (JSON) |
| `POST` | `/auth/logout` | Sign out and start new device flow |
| `GET` | `/v1/models` | List available Copilot models |
| `POST` | `/v1/chat/completions` | Chat completions (streaming + non-streaming) |
| `POST` | `/v1/responses` | Responses API (Codex CLI compatible) |
| `POST` | `/v1/responses/compact` | Context compaction for long sessions |
| `POST` | `/v1/embeddings` | Text embeddings |

## Configuration

All settings can be set via CLI flags, environment variables, or both. CLI flags take priority.

| Setting | CLI Flag | Env Var | Default |
|---------|----------|---------|---------|
| Host | `--host` | `COPILOT_PROXY_HOST` | `0.0.0.0` |
| Port | `--port` | `COPILOT_PROXY_PORT` | `6789` |
| API Key | `--api-key` | `COPILOT_PROXY_API_KEY` | none |
| Log Level | `--log-level` | `COPILOT_PROXY_LOG_LEVEL` | `info` |
| Token Dir | `--token-dir` | `COPILOT_PROXY_TOKEN_DIR` | `~/.config/copilot-proxy/` |

### Examples

```bash
# Custom port with API key protection
copilot-proxy --port 9000 --api-key my-secret-key

# Debug logging
copilot-proxy --log-level debug

# Custom token storage
copilot-proxy --token-dir /tmp/copilot-tokens
```

## How It Works

1. **Authentication** — GitHub OAuth Device Flow. You authorize once in the browser, and the proxy caches your GitHub access token and Copilot API token to `~/.config/copilot-proxy/`.

2. **Token lifecycle** — On each request, the proxy checks if the Copilot token is still valid. If expired, it silently refreshes using the cached GitHub token. If that fails, it triggers a new device flow.

3. **Request forwarding** — Incoming OpenAI-format requests are forwarded to `api.githubcopilot.com` with the required Copilot headers (editor version, plugin version, request ID, etc.).

4. **Streaming** — SSE streams are passed through byte-for-byte with no buffering.

## License

[MIT](LICENSE) - An Nguyen
