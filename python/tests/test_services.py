"""Tests for business logic services."""

from __future__ import annotations

import pytest
from ep_ai.providers.base import LLMProvider
from ep_ai.services.conversation import ConversationService, SessionNotFoundError
from ep_ai.services.scenario import get_scenario, list_scenarios


def test_list_scenarios():
    scenarios = list_scenarios()
    ids = [s["id"] for s in scenarios]
    assert "restaurant" in ids
    assert "introduction" in ids
    assert "shopping" in ids


def test_get_scenario_fallback():
    s = get_scenario("nonexistent")
    assert s == get_scenario("restaurant")  # falls back


def test_get_scenario_restaurant():
    s = get_scenario("restaurant")
    assert "name" in s
    assert "opening" in s
    assert "system_prompt" in s


@pytest.mark.asyncio
async def test_conversation_service_start():
    """Start a conversation and verify session creation."""
    from ep_ai.providers.base import LLMProvider

    class FakeProvider(LLMProvider):
        @property
        def name(self):
            return "fake"

        async def chat(self, messages, **kwargs):
            return "fake reply"

        async def tts(self, text, voice, style=None):
            return b"FAKE_AUDIO"

        async def transcribe_audio(self, audio_bytes, mime_type="audio/wav"):
            return "fake transcript"

        async def chat_with_audio(self, messages, audio_bytes, mime_type="audio/wav", **kwargs):
            return "fake transcript", "fake reply"

    svc = ConversationService(provider=FakeProvider())
    result = await svc.start("restaurant")

    assert "session_id" in result
    assert result["ai_text"] == get_scenario("restaurant")["opening"]
    assert result["audio_base64"] == "RkFLRV9BVURJTw=="  # base64 of b"FAKE_AUDIO"
    assert result["scenario_name"] == "Restaurant Order"


@pytest.mark.asyncio
async def test_conversation_turn_and_respond():
    class FakeProvider(LLMProvider):
        @property
        def name(self):
            return "fake"

        async def chat(self, messages, **kwargs):
            return "text reply"

        async def tts(self, text, voice, style=None):
            return b"AUDIO"

        async def transcribe_audio(self, audio_bytes, mime_type="audio/wav"):
            return "transcript"

        async def chat_with_audio(self, messages, audio_bytes, mime_type="audio/wav", **kwargs):
            return "transcript", "audio reply"

    svc = ConversationService(provider=FakeProvider())
    start = await svc.start("introduction")
    sid = start["session_id"]

    turn = await svc.turn(sid, b"fake_audio")
    assert turn["transcript"] == "transcript"
    assert turn["ai_text"] == "text reply"

    text_turn = await svc.respond_text(sid, "hello")
    assert text_turn["transcript"] == "hello"
    assert text_turn["ai_text"] == "text reply"


@pytest.mark.asyncio
async def test_session_not_found():
    svc = ConversationService()
    with pytest.raises(SessionNotFoundError):
        await svc.turn("bad-session", b"audio")
