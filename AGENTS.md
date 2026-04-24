# English Partner - Agent Guide

## Project Overview

English Partner is an AI-powered English speaking practice application targeting **primary and middle school students** in China. The app simulates conversational scenarios with an AI partner, providing real-time speech interaction, feedback, and gamified learning experiences.

**Long-term Goal**: Deploy on embedded/hardware devices (smart speakers, learning tablets, etc.), requiring minimal latency and efficient resource usage.

## Tech Stack Principles

1. **Rust-first for performance-critical paths** - Audio I/O, WebSocket streaming, VAD, real-time pipelines
2. **Python for AI orchestration** - LLM integration, prompt engineering, curriculum design, data analytics
3. **Multi-language bridge** - gRPC / HTTP + shared protobuf schemas between Rust and Python
4. **Cloud-to-local evolution** - Start with Soniox cloud APIs, gradually migrate to on-device models

## Directory Structure

```
english_partner/
├── crates/               # Rust workspace
│   ├── ep-core/         # Shared types, errors, protobuf definitions
│   ├── ep-audio/        # Audio capture/playback (cpal, rodio)
│   ├── ep-soniox/       # Soniox REST + WebSocket client (no official Rust SDK)
│   ├── ep-vad/          # Voice Activity Detection
│   └── ep-app/          # Main application (Tauri/desktop or embedded)
├── python/
│   ├── ep_ai/           # Python AI service (FastAPI)
│   │   ├── api/         # HTTP endpoints
│   │   ├── services/    # LLM, assessment, curriculum logic
│   │   └── prompts/     # System prompts for scenarios
│   └── scripts/         # Prototyping and data processing
├── proto/               # gRPC/protobuf schemas
├── assets/              # Audio samples, prompt templates
└── docs/                # Architecture decisions, API references
```

## Coding Conventions

- **Rust**: `snake_case` for functions/variables, `PascalCase` for types, comprehensive error handling with `thiserror`
- **Python**: PEP 8, type hints mandatory, `pydantic` for data validation
- **Protobuf**: `snake_case` fields, package `ep.v1`
- **Async-first**: Both Rust (`tokio`) and Python (`asyncio`/`anyio`) use async patterns for I/O

## Soniox API Configuration

- API Key: Loaded from `SONIOX_API_KEY` environment variable (already set in `~/.zshrc`)
- Base URL: `https://api.soniox.com` (default US region)
- TTS Model: `tts-rt-v1-preview`
- STT Model: `stt-rt-v4` (real-time) or `stt-async-v4` (batch)
- TTS Voice: `Adrian` (English) - verify Chinese voices via API

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
- Audio format preference: 16kHz mono 16-bit PCM for STT, WAV for TTS
