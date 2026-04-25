"""OpenAI / Azure-compatible LLM provider (fallback / alternative)."""

from __future__ import annotations

import os
from typing import Any

from openai import OpenAI

from .base import LLMProvider, LLMProviderError

_DEFAULT_MODEL = "gpt-4o-mini"
_DEFAULT_TTS_VOICE = "alloy"


def _get_client() -> OpenAI:
    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        raise LLMProviderError("OPENAI_API_KEY not set. Please configure it in .env")
    return OpenAI(api_key=api_key)


class OpenAIProvider(LLMProvider):
    """OpenAI-native provider: GPT-4o for chat, Whisper for STT, TTS for speech."""

    @property
    def name(self) -> str:
        return "openai"

    def __init__(self) -> None:
        self._client_instance: OpenAI | None = None

    def _client(self) -> OpenAI:
        if self._client_instance is None:
            self._client_instance = _get_client()
        return self._client_instance

    async def chat(self, messages: list[dict[str, Any]], **kwargs: Any) -> str:
        try:
            completion = self._client().chat.completions.create(
                model=kwargs.get("model", _DEFAULT_MODEL),
                messages=messages,  # type: ignore[arg-type]
                max_completion_tokens=kwargs.get("max_tokens", 200),
                temperature=kwargs.get("temperature", 0.7),
            )
            return completion.choices[0].message.content.strip()
        except Exception as exc:
            raise LLMProviderError(f"OpenAI chat failed: {exc}", cause=exc) from exc

    async def tts(self, text: str, voice: str, style: str | None = None) -> bytes:
        # OpenAI TTS does not support style prompts; we ignore `style`.
        try:
            response = self._client().audio.speech.create(
                model="tts-1",
                voice=voice or _DEFAULT_TTS_VOICE,  # type: ignore[arg-type]
                input=text,
            )
            return response.content
        except Exception as exc:
            raise LLMProviderError(f"OpenAI TTS failed: {exc}", cause=exc) from exc

    async def transcribe_audio(self, audio_bytes: bytes, mime_type: str = "audio/wav") -> str:
        import io

        try:
            # Whisper expects file-like objects.
            file_ext = "wav" if "wav" in mime_type else "mp3"
            buffer = io.BytesIO(audio_bytes)
            buffer.name = f"audio.{file_ext}"
            transcript = self._client().audio.transcriptions.create(
                model="whisper-1",
                file=buffer,
            )
            return transcript.text
        except Exception as exc:
            raise LLMProviderError(f"OpenAI transcription failed: {exc}", cause=exc) from exc

    async def chat_with_audio(
        self,
        messages: list[dict[str, Any]],
        audio_bytes: bytes,
        mime_type: str = "audio/wav",
        **kwargs: Any,
    ) -> tuple[str, str]:
        # OpenAI does not have native audio-in for GPT-4o-mini.
        # Fallback: transcribe first, then chat.
        transcript = await self.transcribe_audio(audio_bytes, mime_type)
        chat_messages = list(messages)
        chat_messages.append({"role": "user", "content": f"The student said: '{transcript}'"})
        response = await self.chat(chat_messages, **kwargs)
        return transcript, response
