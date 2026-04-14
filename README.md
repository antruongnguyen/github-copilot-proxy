# copilot-proxy

An OpenAI-compatible HTTP proxy for GitHub Copilot. Authenticate once with your GitHub account, then use any OpenAI-compatible tool (Codex CLI, aider, Continue, curl) against `http://localhost:8080/v1/` — requests are transparently forwarded to GitHub Copilot's API.

## Quick Start

```bash
# Build
cargo build --release

# Run (first time will prompt GitHub device flow auth)
./target/release/copilot-proxy

# In another terminal — test it
curl http://localhost:8080/v1/models

curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

On first run, the proxy will display a URL and code for GitHub device flow authentication. Visit the URL, enter the code, and the proxy will cache your tokens for subsequent runs.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| GET | `/v1/models` | List available Copilot models |
| POST | `/v1/chat/completions` | Chat completions (streaming + non-streaming) |
| POST | `/v1/responses` | OpenAI Responses API (streaming SSE) |
| POST | `/v1/responses/compact` | Compact long conversation context |
| POST | `/v1/embeddings` | Text embeddings |

## Configuration

| Setting | CLI Flag | Env Var | Default |
|---------|----------|---------|---------|
| Host | `--host` | `COPILOT_PROXY_HOST` | `127.0.0.1` |
| Port | `--port` | `COPILOT_PROXY_PORT` | `8080` |
| API Key | `--api-key` | `COPILOT_PROXY_API_KEY` | none (no auth) |
| Log Level | `--log-level` | `COPILOT_PROXY_LOG_LEVEL` | `info` |
| Token Dir | `--token-dir` | `COPILOT_PROXY_TOKEN_DIR` | `~/.config/copilot-proxy/` |

Setting an API key enables local authentication — clients must send `Authorization: Bearer <key>`.

## Codex CLI

The proxy supports [Codex CLI](https://github.com/openai/codex) via the `/v1/responses` endpoint.

```bash
# Auto-configure Codex to use the proxy
./scripts/setup-codex.sh

# Or with custom port/model
./scripts/setup-codex.sh --port 9090 --model o4-mini

# Use Codex with the proxy running
codex "your prompt"
```

The setup script writes `~/.codex/config.toml` with `wire_api = "responses"` pointed at the proxy.

## Available Models

- gpt-4o, gpt-4o-mini
- gpt-4.1, gpt-4.1-mini
- o3-mini, o4-mini
- claude-3.5-sonnet, claude-sonnet-4
- gemini-2.0-flash-001
