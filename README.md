# English Partner 🤖🇬🇧

AI-powered English speaking practice for Chinese primary & middle school students.

## Vision

Build a conversational AI companion that helps kids practice English through natural, engaging dialogues — from ordering food at a restaurant to introducing themselves at school. Eventually deployable on low-cost embedded hardware.

## Key Features (Planned)

- 🎙️ **Real-time Voice Conversations** - Talk naturally, AI responds instantly
- 📚 **Scenario-based Practice** - Restaurant, airport, classroom, shopping, etc.
- 🎯 **Pronunciation Feedback** - Identify weak sounds and provide corrections
- 🏆 **Gamification** - Points, badges, progress tracking for kids
- 🌐 **Bilingual Support** - Chinese instructions + English practice, seamless switching

## Architecture

```
┌─────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Student   │────▶│  Rust Audio Core │────▶│  Soniox Cloud   │
│  (Mic/Speaker│     │  (ep-audio/vad)  │     │  STT + TTS      │
└─────────────┘     └─────────────────┘     └─────────────────┘
                            │
                            ▼
                    ┌─────────────────┐
                    │  Rust App Core  │
                    │  (ep-app)       │
                    └─────────────────┘
                            │
                            ▼
                    ┌─────────────────┐
                    │  Python AI Svc  │
                    │  (ep_ai)        │
                    │  LLM + Curriculum│
                    └─────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.84+
- Python 3.10+
- `SONIOX_API_KEY` in environment

### Setup

```bash
# Python environment
source .venv/bin/activate
uv pip install -e ".[dev]"

# Rust build
cargo build --release

# Run AI service (Python)
python -m ep_ai.main

# Run app (Rust)
cargo run -p ep-app
```

## Tech Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Audio I/O | Rust + `cpal` | Zero-copy, low latency, cross-platform |
| Speech API | Soniox (cloud) | Best-in-class STT/TTS, 60+ languages |
| VAD | Rust + `webrtc-vad` or custom | Detect speech boundaries |
| App Framework | Tauri (future) or egui | Rust-native GUI |
| LLM | OpenAI GPT-4o / Claude | Conversation generation |
| Assessment | Azure Pronunciation (future) | Phoneme-level scoring |
| Protocol | gRPC + Protobuf | Efficient Rust↔Python bridge |

## License

MIT
# english_partner
