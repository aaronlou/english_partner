"""Base abstraction for LLM + speech providers.

The `LLMProvider` interface decouples the business logic (conversation,
assessment, curriculum) from the underlying model vendor (MiMo, OpenAI,
Claude, local models, etc.).

To add a new provider:
1. Subclass `LLMProvider`.
2. Implement all abstract methods.
3. Register it in `ep_ai.providers.__init__`.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class LLMProviderError(Exception):
    """Raised when a provider API call fails."""

    def __init__(self, message: str, cause: Exception | None = None) -> None:
        super().__init__(message)
        self.cause = cause


class LLMProvider(ABC):
    """Unified interface for LLM, TTS, and audio transcription services."""

    @property
    @abstractmethod
    def name(self) -> str:
        """Human-readable provider name."""

    @abstractmethod
    async def chat(self, messages: list[dict[str, Any]], **kwargs: Any) -> str:
        """Send messages to the chat model and return the response text.

        Args:
            messages: OpenAI-compatible message list.
            **kwargs: Provider-specific overrides (temperature, max_tokens, etc.).

        Returns:
            The model's response text.

        Raises:
            LLMProviderError: On API failures.
        """

    @abstractmethod
    async def tts(self, text: str, voice: str, style: str | None = None) -> bytes:
        """Synthesize text into speech audio.

        Args:
            text: Text to speak.
            voice: Voice identifier (provider-specific).
            style: Optional speaking style / personality prompt.

        Returns:
            WAV audio bytes.

        Raises:
            LLMProviderError: On API failures.
        """

    @abstractmethod
    async def transcribe_audio(self, audio_bytes: bytes, mime_type: str = "audio/wav") -> str:
        """Transcribe audio to text.

        Args:
            audio_bytes: Raw audio file bytes.
            mime_type: Audio format hint.

        Returns:
            Transcribed text.

        Raises:
            LLMProviderError: On API failures.
        """

    @abstractmethod
    async def chat_with_audio(
        self,
        messages: list[dict[str, Any]],
        audio_bytes: bytes,
        mime_type: str = "audio/wav",
        **kwargs: Any,
    ) -> tuple[str, str]:
        """Send audio + context to the model; return (transcript, response).

        This is a convenience method for the common conversation turn where
        the student speaks and the AI needs to both understand and reply.

        Returns:
            (transcript, ai_response_text)

        Raises:
            LLMProviderError: On API failures.
        """
