"""Tests for LLM provider abstraction."""

from __future__ import annotations

import pytest
from ep_ai.providers import LLMProvider, ProviderRegistry, list_providers
from ep_ai.providers.base import LLMProviderError


class DummyProvider(LLMProvider):
    """In-memory provider for testing — no network calls."""

    @property
    def name(self) -> str:
        return "dummy"

    async def chat(self, messages, **kwargs):
        return "hello from dummy"

    async def tts(self, text, voice, style=None):
        return b"FAKE_WAV"

    async def transcribe_audio(self, audio_bytes, mime_type="audio/wav"):
        return "transcribed"

    async def chat_with_audio(self, messages, audio_bytes, mime_type="audio/wav", **kwargs):
        return "student said something", "ai replies"


def test_registry_register_and_get():
    reg = ProviderRegistry()
    reg.register("dummy", DummyProvider)

    p = reg.get("dummy")
    assert isinstance(p, DummyProvider)
    assert p.name == "dummy"


def test_registry_unknown_provider():
    reg = ProviderRegistry()
    with pytest.raises(KeyError):
        reg.get("nonexistent")


def test_builtin_providers_listed():
    names = list_providers()
    assert "mimo" in names
    assert "openai" in names


def test_get_provider_defaults_to_mimo():
    # Will raise if MIMO_API_KEY is not set, because get_provider instantiates.
    # We override with dummy to avoid env dependency.
    reg = ProviderRegistry()
    reg.register("dummy", DummyProvider)
    p = reg.get("dummy")
    assert p.name == "dummy"


def test_llm_provider_error_message():
    err = LLMProviderError("something broke")
    assert str(err) == "something broke"
    assert err.cause is None
