# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-14

### Added
- OpenAI-compatible proxy endpoints: `/v1/chat/completions`, `/v1/embeddings`, `/v1/models`
- GitHub OAuth Device Flow authentication with automatic token caching
- Copilot API token auto-refresh on expiration
- SSE streaming pass-through for real-time responses
- Web UI at `/` showing authentication status, device code, and available models
- `POST /auth/logout` endpoint for account switching
- Optional API key middleware for local client authentication
- File-based token persistence at `~/.config/copilot-proxy/`
- Dynamic model listing from GitHub Copilot API
- Graceful shutdown on SIGINT/SIGTERM
- Health check endpoint at `/health`
- Browser auto-open only when authentication is needed
- 29 unit and integration tests
