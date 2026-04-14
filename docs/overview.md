# Project Overview

## What is copilot-proxy?

copilot-proxy is a lightweight HTTP proxy written in Rust that makes GitHub Copilot's AI models accessible through a standard OpenAI-compatible API. It bridges the gap between GitHub Copilot (which normally requires IDE plugins) and the broader ecosystem of OpenAI-compatible tools.

## Problem

GitHub Copilot provides access to powerful AI models (GPT-5.4, Claude, Gemini, and more) but locks them behind IDE-specific plugins. If you want to use these models from CLI tools, custom scripts, or alternative editors, there's no official way to do it.

## Solution

copilot-proxy runs locally and:

1. Authenticates with GitHub using the standard OAuth Device Flow
2. Manages Copilot API tokens (caching, auto-refresh)
3. Translates OpenAI-format requests into Copilot API calls
4. Streams responses back to any client that speaks the OpenAI protocol

## Architecture

```
Client (curl, aider, codex, etc.)
    │
    │  OpenAI-format HTTP request
    ▼
┌──────────────────────────────┐
│        copilot-proxy         │
│                              │
│  ┌────────┐  ┌────────────┐ │
│  │ Router │──│ Auth       │ │
│  │ (axum) │  │ Manager    │ │
│  └───┬────┘  └─────┬──────┘ │
│      │             │        │
│  ┌───▼────┐  ┌─────▼──────┐ │
│  │ Proxy  │  │ Token      │ │
│  │Handler │  │ Storage    │ │
│  └───┬────┘  └────────────┘ │
│      │                      │
└──────┼──────────────────────┘
       │
       │  Copilot-format HTTP request
       ▼
  api.githubcopilot.com
```

## Key Design Decisions

- **Rust** — chosen for performance, safety, and small binary size. The proxy adds sub-millisecond overhead.
- **axum** — async HTTP framework with excellent middleware support, same as used in production proxies.
- **File-based token storage** — tokens cached at `~/.config/copilot-proxy/` for simplicity and debuggability. No keyring dependency.
- **GitHub Device Flow** — the same auth flow used by GitHub CLI (`gh`). No client secret needed.
- **Pass-through streaming** — SSE streams from Copilot are forwarded byte-for-byte with no buffering, ensuring real-time token delivery.
- **Web UI for auth** — instead of requiring terminal interaction, the proxy serves a browser page showing the device code and a button to open GitHub.

## What It Is Not

- Not a GitHub Copilot replacement — it requires an active Copilot subscription
- Not a model host — it proxies to GitHub's infrastructure
- Not an API key generator — it uses GitHub's OAuth flow for legitimate authentication
