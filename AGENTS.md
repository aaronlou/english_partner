# English Partner - Agent Guide

## Project Overview

English Partner is an AI-powered English speaking practice application targeting **primary and middle school students** in China. The app simulates conversational scenarios with an AI partner, providing real-time speech interaction, feedback, and gamified learning experiences.

**Long-term Goal**: Deploy on embedded/hardware devices (smart speakers, learning tablets, etc.), requiring minimal latency and efficient resource usage.

## Tech Stack Principles

1. **Rust-first for performance-critical paths** - Audio I/O, WebSocket streaming, VAD, real-time pipelines
2. **Python for AI orchestration** - LLM integration, prompt engineering, curriculum design, data analytics
3. **Rust for real-time voice** - ep-app is a standalone voice app using Volcengine Realtime API (no Python dependency)
4. **Cloud-to-local evolution** - Start with Volcengine cloud APIs, gradually migrate to on-device models

## Directory Structure

```
english_partner/
├── crates/               # Rust workspace
│   ├── ep-core/         # Shared types, errors, protobuf definitions
│   ├── ep-audio/        # Audio capture/playback (cpal, rodio)
│   ├── ep-soniox/       # Soniox REST + WebSocket client (legacy/deprecated)
│   ├── ep-volcengine/   # Volcengine Realtime Speech API client (active)
│   ├── ep-vad/          # Voice Activity Detection
│   └── ep-app/          # Main voice application
├── python/
│   ├── ep_ai/           # Python AI service (FastAPI, text-only fallback)
│   │   ├── api/         # HTTP endpoints
│   │   ├── services/    # LLM, assessment, curriculum logic
│   │   ├── providers/   # LLM provider abstraction (OpenAI, MiMo)
│   │   └── prompts/     # System prompts for scenarios
│   └── scripts/         # Prototyping and data processing
├── cli_*.py              # Standalone CLI clients (text + voice)
├── docs/                # Architecture decisions, API references
└── target/              # Compiled Rust binaries
```

## Coding Conventions

- **Rust**: `snake_case` for functions/variables, `PascalCase` for types, comprehensive error handling with `thiserror`
- **Python**: PEP 8, type hints mandatory, `pydantic` for data validation
- **Async-first**: Both Rust (`tokio`) and Python (`asyncio`/`anyio`) use async patterns for I/O

## Volcengine Realtime API Configuration

- API Endpoint: `wss://openspeech.bytedance.com/api/v3/realtime/dialogue`
- App ID: `VOLCANO_APPID` env var (from 火山引擎控制台)
- Access Token: `VOLCANO_ACCESS_TOKEN` env var
- App Key: `VOLCANO_APP_KEY` env var (default: `PlgvMymc7f3tQnJ6`)
- Resource ID: `volc.speech.dialog`
- Model: `1.2.1.1` (O2.0 version — supports system_role + speaking_style)
- Audio Input: PCM 16kHz mono 16-bit LE, 20ms chunks (640 bytes)
- Audio Output: PCM 24kHz mono 16-bit LE (pcm_s16le)
- Voice: `zh_female_vv_jupiter_bigtts` (vv, lively female voice)
- Protocol: Custom binary framing over WebSocket (see ep-volcengine/src/binary.rs)

## Build Commands

```bash
# Rust
cargo build --release
cargo test
cargo clippy

# Python
source .venv/bin/activate
uv pip install -e python/ep_ai
python -m pytest python/

# Full stack (dev)
docker-compose up  # (future)
```

## Notes

- No `git commit` / `git push` without explicit user confirmation
- Keep hardware constraints in mind: target ~100MB RAM, ARM Cortex-A53 class CPU for Phase 3
- Audio format: 16kHz mono 16-bit PCM for input, 24kHz mono 16-bit PCM for output
- Running the app: `cargo run -p ep-app` (requires VOLCANO_APPID + VOLCANO_ACCESS_TOKEN env vars)
- Python AI service is text-only fallback; primary voice path is Rust-only via Volcengine
