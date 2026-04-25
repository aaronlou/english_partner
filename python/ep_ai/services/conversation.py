"""Conversation session management."""

from __future__ import annotations

import asyncio
import os
import tempfile
import uuid
from typing import Any

import structlog

from ep_ai.providers import get_provider
from ep_ai.providers.base import LLMProvider
from ep_ai.services.scenario import get_scenario

logger = structlog.get_logger()

# In-memory session store (sufficient for single-user MVP).
# Future: replace with Redis or a small DB.
_sessions: dict[str, dict[str, Any]] = {}

_MAX_HISTORY = 20  # 10 turns (student + AI per turn)


class ConversationService:
    """Manages conversation lifecycle: start, turn, text fallback."""

    def __init__(self, provider: LLMProvider | None = None) -> None:
        self.provider = provider or get_provider()
        self._stt_model: Any = None

    def _get_stt(self) -> Any:
        """Lazy-load local Whisper model for STT."""
        if self._stt_model is None:
            import whisper

            logger.info("whisper.loading_model")
            # 'base' is ~150MB, good balance of speed/accuracy for English
            # M4 Max runs this in ~0.3s for a 5s utterance
            self._stt_model = whisper.load_model("base")
            logger.info("whisper.model_loaded")
        return self._stt_model

    async def start(self, scenario_id: str) -> dict[str, Any]:
        """Start a new conversation scenario.

        Returns:
            {"session_id", "ai_text", "audio_base64", "scenario_name"}
        """
        scenario = get_scenario(scenario_id)
        session_id = str(uuid.uuid4())[:8]
        opening = scenario["opening"]

        audio_bytes = await self.provider.tts(opening, voice="Chloe")
        audio_b64 = __import__("base64").b64encode(audio_bytes).decode()

        _sessions[session_id] = {
            "scenario_id": scenario_id,
            "system_prompt": scenario["system_prompt"],
            "history": [{"role": "ai", "text": opening}],
        }

        logger.info(
            "conversation.started",
            session_id=session_id,
            scenario=scenario_id,
            provider=self.provider.name,
        )
        return {
            "session_id": session_id,
            "ai_text": opening,
            "audio_base64": audio_b64,
            "scenario_name": scenario["name"],
        }

    async def turn(self, session_id: str, audio_bytes: bytes) -> dict[str, Any]:
        """Process one audio turn: student speaks -> AI responds.

        Uses local Whisper for STT (MiMo does not support audio-in),
        then text-based LLM + TTS.
        """
        session = _require_session(session_id)

        logger.info(
            "conversation.turn",
            session_id=session_id,
            audio_bytes=len(audio_bytes),
        )

        # ── STT: prefer provider transcription, fallback to local Whisper ──
        try:
            transcript = await self.provider.transcribe_audio(audio_bytes)
        except Exception:
            with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
                f.write(audio_bytes)
                tmp_path = f.name

            try:
                model = self._get_stt()
                result = await asyncio.to_thread(
                    model.transcribe, tmp_path, language="en", fp16=False
                )
                transcript = result["text"].strip()
            finally:
                os.unlink(tmp_path)

        logger.info("stt.transcribed", transcript=transcript)

        if not transcript:
            # Empty transcript — return a gentle retry prompt
            ai_text = "I didn't catch that. Could you say it again, please?"
            ai_audio = await self.provider.tts(ai_text, voice="Chloe")
            ai_audio_b64 = __import__("base64").b64encode(ai_audio).decode()
            return {
                "transcript": "",
                "ai_text": ai_text,
                "audio_base64": ai_audio_b64,
            }

        # ── LLM + TTS ──────────────────────────────────────────────
        messages = _build_messages(session)
        messages.append({"role": "user", "content": f"The student said: '{transcript}'"})
        ai_text = await self.provider.chat(messages)
        ai_audio = await self.provider.tts(ai_text, voice="Chloe")
        ai_audio_b64 = __import__("base64").b64encode(ai_audio).decode()

        _append_history(session, transcript, ai_text)

        return {
            "transcript": transcript,
            "ai_text": ai_text,
            "audio_base64": ai_audio_b64,
        }

    async def respond_text(self, session_id: str, student_text: str) -> dict[str, Any]:
        """Text-only fallback (no audio input)."""
        session = _require_session(session_id)

        messages = _build_messages(session)
        messages.append({"role": "user", "content": f"The student said: '{student_text}'"})

        ai_text = await self.provider.chat(messages)
        ai_audio = await self.provider.tts(ai_text, voice="Chloe")
        ai_audio_b64 = __import__("base64").b64encode(ai_audio).decode()

        _append_history(session, student_text, ai_text)

        return {
            "transcript": student_text,
            "ai_text": ai_text,
            "audio_base64": ai_audio_b64,
        }

    def health(self) -> dict[str, Any]:
        """Return provider connectivity status."""
        ok = True
        try:
            # Quick sanity check: just ensure provider instantiation succeeded.
            _ = self.provider.name
        except Exception:
            ok = False
        return {"status": "ok", f"{self.provider.name}_connected": ok}


def _require_session(session_id: str) -> dict[str, Any]:
    session = _sessions.get(session_id)
    if not session:
        raise SessionNotFoundError(session_id)
    return session


def _build_messages(session: dict[str, Any]) -> list[dict[str, Any]]:
    messages: list[dict[str, Any]] = [
        {"role": "system", "content": session["system_prompt"]}
    ]
    for h in session["history"]:
        role = "assistant" if h["role"] == "ai" else "user"
        messages.append({"role": role, "content": h["text"]})
    return messages


def _append_history(session: dict[str, Any], student_text: str, ai_text: str) -> None:
    session["history"].append({"role": "student", "text": student_text})
    session["history"].append({"role": "ai", "text": ai_text})
    if len(session["history"]) > _MAX_HISTORY:
        session["history"] = session["history"][-_MAX_HISTORY:]


class SessionNotFoundError(Exception):
    def __init__(self, session_id: str) -> None:
        super().__init__(f"Session not found: {session_id}")
        self.session_id = session_id
