#!/usr/bin/env bash
set -euo pipefail

# Setup Codex CLI to use copilot-proxy as its provider.
# Usage: scripts/setup-codex.sh [--port PORT] [--model MODEL]

PORT=6789
MODEL="gpt-5.4"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port) PORT="$2"; shift 2 ;;
    --model) MODEL="$2"; shift 2 ;;
    [0-9]*) PORT="$1"; shift ;;
    *) echo "Usage: $0 [--port PORT] [--model MODEL]"; exit 1 ;;
  esac
done

# Check if codex is installed
if ! command -v codex &>/dev/null; then
  echo "Codex CLI not found. Install it first:"
  echo "  npm install -g @openai/codex"
  exit 1
fi

CONFIG_DIR="${HOME}/.codex"
CONFIG_FILE="${CONFIG_DIR}/config.toml"

mkdir -p "$CONFIG_DIR"

# Write config — always overwrite the copilot-proxy section.
# If the file exists, preserve lines that are NOT part of the top-level model/model_provider
# or the [model_providers.copilot-proxy] block. In practice, Codex config is small and
# copilot-proxy is the primary use case, so we write the full file.
cat > "$CONFIG_FILE" <<EOF
model = "${MODEL}"
model_provider = "copilot-proxy"

[model_providers.copilot-proxy]
name = "GitHub Copilot via copilot-proxy"
base_url = "http://127.0.0.1:${PORT}/v1"
wire_api = "responses"
env_key = "COPILOT_PROXY_API_KEY"
EOF

echo "Codex configured at ${CONFIG_FILE}"
echo ""
echo "  model:          ${MODEL}"
echo "  provider:       copilot-proxy"
echo "  proxy URL:      http://127.0.0.1:${PORT}/v1"
echo "  wire_api:       responses"
echo ""

if [[ -n "${COPILOT_PROXY_API_KEY:-}" ]]; then
  echo "  COPILOT_PROXY_API_KEY is set in your environment."
else
  echo "  No COPILOT_PROXY_API_KEY set. None needed unless proxy was started with --api-key."
fi

echo ""
echo "Usage:"
echo "  codex \"your prompt\""
echo "  codex --provider copilot-proxy \"your prompt\""
