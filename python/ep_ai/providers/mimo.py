"""MiMo (Xiaomi) LLM provider implementation."""

from __future__ import annotations

import base64
import os
import re
from typing import Any

from openai import OpenAI

from .base import LLMProvider, LLMProviderError

_API_BASE = "https://api.xiaomimimo.com/v1"
_TTS_MODEL = "mimo-v2.5-tts"
_CHAT_MODEL = "mimo-v2.5"
_DEFAULT_VOICE = "Chloe"
_DEFAULT_STYLE = (
    "Warm, enthusiastic elementary school teacher tone. "
    "Speak clearly and slowly with lots of encouragement, "
    "like you're talking to a 10-year-old student."
)


def _parse_transcript_response(content: str) -> tuple[str, str]:
    """Extract transcript and response from MiMo's combined output.

    Tries multiple strategies in order of preference.
    """
    # Strategy 1: explicit [TRANSCRIPT]...[/TRANSCRIPT] tags
    t_match = re.search(
        r"\[TRANSCRIPT\]\s*(.*?)\s*\[/TRANSCRIPT\]",
        content,
        re.DOTALL | re.IGNORECASE,
    )
    r_match = re.search(
        r"\[RESPONSE\]\s*(.*?)\s*\[/RESPONSE\]",
        content,
        re.DOTALL | re.IGNORECASE,
    )
    if t_match and r_match:
        return t_match.group(1).strip(), r_match.group(1).strip()

    # Strategy 2: markdown **Transcription:** ... followed by response
    content_clean = content.replace("**", "")
    t_md = re.search(r"Transcription:\s*['\"]?([^\n]+)['\"]?", content_clean, re.IGNORECASE)
    if t_md:
        transcript = t_md.group(1).strip().strip('"').strip("'")
        # Response is everything after the first blank line following Transcription
        parts = re.split(r"\n\s*\n", content_clean, maxsplit=1)
        if len(parts) == 2:
            return transcript, parts[1].strip()
        return transcript, content_clean

    # Strategy 3: split by double newline
    parts = content.split("\n\n", 1)
    if len(parts) == 2:
        return parts[0].strip(), parts[1].strip()

    # Fallback: return same text for both
    return content, content


def _get_client() -> OpenAI:
    api_key = os.environ.get("MIMO_API_KEY")
    if not api_key or api_key == "placeholder":
        raise LLMProviderError("MIMO_API_KEY not set. Please configure it in .env")
    return OpenAI(api_key=api_key, base_url=_API_BASE)


class MiMoProvider(LLMProvider):
    """MiMo v2.5 provider: chat, TTS, and audio understanding."""

    @property
    def name(self) -> str:
        return "mimo"

    def __init__(self) -> None:
        self._client_instance: OpenAI | None = None

    def _client(self) -> OpenAI:
        if self._client_instance is None:
            self._client_instance = _get_client()
        return self._client_instance

    async def chat(self, messages: list[dict[str, Any]], **kwargs: Any) -> str:
        try:
            completion = self._client().chat.completions.create(
                model=_CHAT_MODEL,
                messages=messages,  # type: ignore[arg-type]
                max_completion_tokens=kwargs.get("max_tokens", 200),
            )
            return completion.choices[0].message.content.strip()
        except Exception as exc:
            raise LLMProviderError(f"MiMo chat failed: {exc}", cause=exc) from exc

    async def tts(self, text: str, voice: str, style: str | None = None) -> bytes:
        style_text = style or _DEFAULT_STYLE
        try:
            completion = self._client().chat.completions.create(
                model=_TTS_MODEL,
                messages=[
                    {"role": "user", "content": style_text},
                    {"role": "assistant", "content": text},
                ],
                audio={"format": "wav", "voice": voice or _DEFAULT_VOICE},
            )
            audio_data = completion.choices[0].message.audio.data
            return base64.b64decode(audio_data)
        except Exception as exc:
            raise LLMProviderError(f"MiMo TTS failed: {exc}", cause=exc) from exc

    async def transcribe_audio(self, audio_bytes: bytes, mime_type: str = "audio/wav") -> str:
        # MiMo does not have a standalone transcription endpoint;
        # we use the audio-understanding chat path with a transcription-only prompt.
        audio_b64 = base64.b64encode(audio_bytes).decode()
        data_uri = f"data:{mime_type};base64,{audio_b64}"
        messages = [
            {
                "role": "user",
                "content": [
                    {"type": "input_audio", "input_audio": {"data": data_uri}},
                    {
                        "type": "text",
                        "text": "Transcribe the audio exactly. Return only the spoken text.",
                    },
                ],
            }
        ]
        return await self.chat(messages, max_tokens=300)

    async def chat_with_audio(
        self,
        messages: list[dict[str, Any]],
        audio_bytes: bytes,
        mime_type: str = "audio/wav",
        **kwargs: Any,
    ) -> tuple[str, str]:
        audio_b64 = base64.b64encode(audio_bytes).decode()
        data_uri = f"data:{mime_type};base64,{audio_b64}"

        full_messages = list(messages)
        full_messages.append({
            "role": "user",
            "content": [
                {"type": "input_audio", "input_audio": {"data": data_uri}},
                {
                    "type": "text",
                    "text": (
                        "The student responded with this audio. "
                        "Please transcribe what they said (in English or Chinese), "
                        "then respond naturally as the English teacher. "
                        "Keep your response to 1-3 short sentences suitable for a child.\n\n"
                        "Format your reply EXACTLY like this (no markdown, no extra text):\n"
                        "[TRANSCRIPT]\n"
                        "the student's exact words\n"
                        "[/TRANSCRIPT]\n\n"
                        "[RESPONSE]\n"
                        "your reply as the teacher\n"
                        "[/RESPONSE]"
                    ),
                },
            ],
        })

        try:
            completion = self._client().chat.completions.create(
                model=_CHAT_MODEL,
                messages=full_messages,  # type: ignore[arg-type]
                max_completion_tokens=kwargs.get("max_tokens", 300),
            )
            content = completion.choices[0].message.content.strip()
            transcript, response = _parse_transcript_response(content)
            return transcript, response
        except Exception as exc:
            raise LLMProviderError(f"MiMo audio chat failed: {exc}", cause=exc) from exc
