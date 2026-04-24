# English Partner - Technical Architecture

## 1. Product Definition

**Target Users**: Chinese primary & middle school students (ages 7-15)

**Core Value Proposition**: A conversational AI partner that makes English speaking practice fun, engaging, and accessible — simulating real-world scenarios (restaurant, school, shopping, airport) with immediate feedback.

**Key Differentiators**:
- Real-time voice interaction (not text-based chat)
- Scenario-based learning (contextual, not random Q&A)
- Pronunciation feedback (identify specific sound problems)
- Bilingual scaffolding (Chinese hints when stuck)
- Gamified progress tracking

---

## 2. Soniox API Capability Analysis

### 2.1 What Soniox Provides

| Capability | API | Quality | Relevance |
|-----------|-----|---------|-----------|
| **English TTS** | REST + WebSocket | Native-speaker, low latency | ⭐⭐⭐ Core - AI speaks to student |
| **English STT** | Real-time WebSocket | WER 6.5%, sub-200ms | ⭐⭐⭐ Core - understands student |
| **Chinese STT** | Real-time WebSocket | WER 6.6%, supports Mandarin/Cantonese | ⭐⭐⭐ Core - for Chinese hints/help |
| **Endpoint Detection** | STT WebSocket | Built-in | ⭐⭐⭐ Core - knows when student finishes |
| **Language Switching** | STT auto-detect | Mid-sentence switching | ⭐⭐ Useful - bilingual practice |
| **Alphanumeric Precision** | TTS | Phone numbers, emails exact | ⭐ Relevant for specific scenarios |
| **Context Injection** | STT | Domain vocabulary boost | ⭐⭐ Useful - education terms |

### 2.2 Critical Gaps for Speaking Practice

| Missing Capability | Impact | Mitigation Strategy |
|-------------------|--------|---------------------|
| **Pronunciation Assessment** | Cannot score how well student pronounced words | Phase 1: Text similarity fallback. Phase 2: Integrate Azure Speech Pronunciation Assessment API or SpeechSuper |
| **Phoneme Alignment** | Cannot identify which specific sounds are wrong | Same as above |
| **Fluency Scoring** | No metrics for pace, pauses, filler words | Derive from STT token timestamps + heuristics |
| **Emotion/Prosody** | Cannot evaluate intonation, stress | Out of scope for MVP |

### 2.3 Cost Analysis (Soniox Cloud)

| Service | Rate | Typical Usage/Session | Cost/Session |
|---------|------|----------------------|--------------|
| TTS | $0.70/hour | ~2 min AI speech | ~$0.023 |
| STT | ~$0.12/hour | ~5 min student speech | ~$0.01 |
| **Total** | | 10-min practice | ~$0.03-0.05 |

> At 1000 daily active users, 10 min each: ~$30-50/day. Viable for initial validation.

---

## 3. System Architecture

### 3.1 Three-Phase Evolution

```
Phase 1: Cloud-First MVP (Now - 3 months)
┌──────────────────────────────────────────────────────────────┐
│  Student Device (Mac/PC/Web)                                 │
│  ┌─────────────┐  ┌─────────────────┐  ┌──────────────┐   │
│  │ Python App  │──│ Soniox Cloud    │──│ OpenAI Cloud │   │
│  │ (Streamlit) │  │ (STT + TTS)     │  │ (LLM)        │   │
│  └─────────────┘  └─────────────────┘  └──────────────┘   │
└──────────────────────────────────────────────────────────────┘

Phase 2: Hybrid Performance (3 - 9 months)
┌──────────────────────────────────────────────────────────────┐
│  Student Device                                              │
│  ┌─────────────────┐      ┌─────────────────────────────┐   │
│  │ Rust Audio Core │──────│ Python AI Service (local)   │   │
│  │ - cpal/rodio    │      │ - FastAPI                   │   │
│  │ - WebSocket STT │      │ - LLM via API               │   │
│  │ - WebSocket TTS │      │ - Curriculum logic          │   │
│  └─────────────────┘      └─────────────────────────────┘   │
│           │                          │                       │
│           └──────────┬───────────────┘                       │
│                      ▼                                       │
│           ┌─────────────────┐                                │
│           │ Soniox Cloud    │                                │
│           │ (STT + TTS)     │                                │
│           └─────────────────┘                                │
└──────────────────────────────────────────────────────────────┘

Phase 3: On-Device Edge (9 - 18 months)
┌──────────────────────────────────────────────────────────────┐
│  Embedded Device (ARM Linux, ~100MB RAM)                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Rust Full-Stack                                      │    │
│  │  - cpal (audio)                                     │    │
│  │  - Whisper.cpp (STT, quantized)                     │    │
│  │  - Piper TTS (on-device synthesis)                  │    │
│  │  - llama.cpp / phi-3 (small LLM)                    │    │
│  │  - ep-app (business logic)                          │    │
│  └─────────────────────────────────────────────────────┘    │
│                      │                                       │
│           ┌──────────┴──────────┐                           │
│           ▼                     ▼                           │
│    ┌─────────────┐      ┌─────────────┐                     │
│    │ Local Models│      │ Cloud Sync  │                     │
│    │ (offline)   │      │ (analytics) │                     │
│    └─────────────┘      └─────────────┘                     │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 Module Responsibilities

#### Rust Crates

| Crate | Responsibility | Phase |
|-------|---------------|-------|
| `ep-core` | Shared types (`Utterance`, `Scenario`, `Speaker`), errors, protobuf | All |
| `ep-audio` | Microphone capture (16kHz PCM), speaker playback, format conversion | 2+ |
| `ep-soniox` | HTTP/WebSocket client for Soniox APIs, connection management | 2 |
| `ep-vad` | Voice Activity Detection, speech boundary detection | 2+ |
| `ep-app` | Main event loop, state machine, UI integration | 2+ |

#### Python Services

| Module | Responsibility | Phase |
|--------|---------------|-------|
| `ep_ai.api` | FastAPI endpoints: `/conversation/*`, `/assessment/*` | 1-2 |
| `ep_ai.services.llm` | OpenAI/Claude integration, prompt construction | 1-2 |
| `ep_ai.services.assessment` | Pronunciation scoring, progress tracking | 1-2 |
| `ep_ai.services.curriculum` | Scenario management, difficulty adaptation | 1-2 |
| `ep_ai.prompts` | System prompts for each scenario | 1-2 |

---

## 4. Data Flow

### 4.1 Conversation Turn (Phase 2)

```
1. Student speaks ──▶ [ep-audio: mic capture]
                        │
2. Audio chunks ────▶ [ep-vad: detect speech start]
                        │
3. Speech detected ─▶ [ep-soniox: WebSocket STT stream]
                        │
4. Transcript ──────▶ [ep-app: "I'd like a hamburger"]
                        │
5. HTTP POST ───────▶ [ep_ai: /conversation/respond]
                        │
6. LLM generates ───▶ [ep_ai: "Great choice! Would you like fries with that?"]
                        │
7. TTS request ─────▶ [ep-soniox: WebSocket TTS stream]
                        │
8. Audio chunks ────▶ [ep-audio: speaker playback]
                        │
9. Student hears ───▶ "Great choice! Would you like fries with that?"
```

**Target latency per turn**: < 1.5s end-to-end (STT 200ms + network 300ms + LLM 500ms + TTS 200ms + audio 100ms)

### 4.2 Pronunciation Assessment Flow

```
Student speaks reference sentence ──▶ STT
                                         │
                    [ep_ai: /assessment/pronunciation]
                                         │
                    Compare student_text vs reference_text
                    + Optional: Azure Pronunciation API
                                         │
                    Return: score, weak_phonemes[], feedback
```

---

## 5. Hardware Target Specs (Phase 3)

| Component | Target | Example Platforms |
|-----------|--------|-------------------|
| CPU | ARM Cortex-A53 quad-core 1.2GHz | Raspberry Pi 3, Allwinner H6 |
| RAM | 512MB total, 100MB for app | |
| Storage | 2GB eMMC / SD card | |
| Audio | I2S MEMS microphone + DAC | |
| Network | WiFi (optional, for sync) | |
| OS | Linux (Buildroot/Yocto) or RTOS | |

**On-device model sizes**:
- STT: Whisper.cpp base (~150MB quantized to q5_0 = ~75MB)
- TTS: Piper (~20-50MB per voice)
- LLM: Phi-3 mini (~2GB quantized to q4 = ~1.5GB) — may need cloud fallback

> Note: Full on-device LLM may be too heavy. Consider:
> - Hybrid: Small local LLM for simple responses, cloud for complex ones
> - Pre-scripted responses with templating (no LLM needed)
> - Or target Raspberry Pi 5 (8GB) class hardware

---

## 6. API & Integration Decisions

### 6.1 Soniox API Usage

| Feature | API Choice | Rationale |
|---------|-----------|-----------|
| TTS (AI speaking) | WebSocket streaming | Lowest latency, start playback before sentence complete |
| STT (student speaking) | WebSocket real-time | Word-by-word transcription, immediate endpoint detection |
| Batch audio processing | REST async | For assessment analytics, not real-time |

### 6.2 LLM Choice

| Phase | Model | Rationale |
|-------|-------|-----------|
| Phase 1 | GPT-4o-mini / Claude 3.5 Haiku | Fast, cheap, great at following prompts |
| Phase 2 | Same, via local caching | Cache common responses |
| Phase 3 | Phi-3 / Qwen2.5 / Llama 3.2 3B | Small enough for edge, good Chinese-English |

### 6.3 Pronunciation Assessment Options

| Option | Accuracy | Cost | Latency | Recommendation |
|--------|----------|------|---------|----------------|
| Text similarity (fallback) | Low | Free | Instant | Phase 1 only |
| Azure Speech Pronunciation | High | ~$1/hour | ~500ms | Phase 2 primary |
| SpeechSuper API | Medium-High | ~$0.5/hour | ~300ms | Phase 2 alternative |
| Whisper phoneme alignment | Medium | Free (local) | ~1s | Phase 3 research |
| Self-trained model | ? | R&D cost | Variable | Long-term |

---

## 7. Development Roadmap

### Phase 1: Cloud MVP (Weeks 1-8)

**Goal**: Validate product-market fit with minimal engineering

- [ ] Week 1-2: Python prototype (Streamlit/Gradio)
  - Basic STT + TTS loop with Soniox
  - 3 conversation scenarios
  - Simple text-based pronunciation scoring
- [ ] Week 3-4: LLM integration
  - OpenAI API integration
  - Prompt engineering for kid-friendly tone
  - Conversation state management
- [ ] Week 5-6: Polish UX
  - Audio visualization
  - Scenario selection UI
  - Basic progress tracking
- [ ] Week 7-8: User testing
  - Test with 5-10 students
  - Collect feedback on voice quality, conversation flow
  - Iterate on prompts

**Tech**: Python-only, Soniox Python SDK, OpenAI API, Streamlit

### Phase 2: Rust Core + Hybrid (Weeks 9-24)

**Goal**: Production-ready performance, start hardware preparation

- [ ] Month 3: Rust audio pipeline
  - ep-audio: cpal capture + rodio playback
  - ep-vad: basic VAD implementation
  - Latency benchmarking
- [ ] Month 4: Rust Soniox client
  - ep-soniox: WebSocket STT + TTS
  - Connection resilience, reconnection
  - Audio streaming optimization
- [ ] Month 5: Python AI service hardening
  - FastAPI with structured logging
  - Pronunciation assessment integration
  - Curriculum database
- [ ] Month 6: Integration + Desktop App
  - Rust-Python gRPC bridge
  - Tauri desktop app prototype
  - End-to-end latency optimization

### Phase 3: Edge Deployment (Months 7-18)

**Goal**: Hardware-ready, minimal cloud dependency

- [ ] Whisper.cpp integration (STT)
- [ ] Piper TTS integration
- [ ] Small LLM evaluation and integration
- [ ] Buildroot/Yocto system image
- [ ] Hardware prototype (custom PCB or COTS SBC)
- [ ] Manufacturing preparation

---

## 8. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Soniox pricing changes | Medium | High | Abstract speech interface, swappable provider |
| Chinese TTS quality insufficient | Medium | High | Evaluate early, fallback to Azure/Aliyun TTS |
| Pronunciation assessment accuracy poor | High | High | Set expectations, human-in-the-loop for early users |
| Edge LLM too slow/dumb | Medium | High | Hybrid cloud-local, pre-scripted responses |
| Hardware cost too high | Medium | High | Start with higher-end SBC (RPi 5), cost-reduce later |
| Student engagement drops | High | Critical | Gamification, multiple characters, progress rewards |

---

## 9. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| End-to-end turn latency | < 1.5s | Instrumentation in Rust |
| STT accuracy (English) | > 90% | Manual review of samples |
| Session completion rate | > 70% | Analytics |
| Return rate (DAU/MAU) | > 30% | Analytics |
| Pronunciation score correlation | r > 0.7 vs human rater | A/B testing |
| Edge device cost | < ¥200 (~$30) | BOM analysis |
