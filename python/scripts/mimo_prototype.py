#!/usr/bin/env python3
"""
MiMo 快速原型 - 验证 TTS + Audio Understanding 质量
====================================================

测试项目：
1. TTS 英文质量对比（不同风格、不同音色）
2. Audio Understanding - 英文语音识别精度
3. 端到端对话体验

前置要求：
    export MIMO_API_KEY=your_key
    uv pip install openai soundfile numpy

注册地址：https://platform.xiaomimimo.com/
"""

from __future__ import annotations

import base64
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import numpy as np
import soundfile as sf
from openai import OpenAI

# =============================================================================
# Configuration
# =============================================================================

API_BASE = "https://api.xiaomimimo.com/v1"
OUTPUT_DIR = Path(__file__).parent / "mimo_outputs"
OUTPUT_DIR.mkdir(exist_ok=True)


def get_client() -> OpenAI:
    """Initialize MiMo API client."""
    api_key = os.environ.get("MIMO_API_KEY")
    if not api_key:
        # Try ~/.zshrc
        zshrc = Path.home() / ".zshrc"
        if zshrc.exists():
            for line in zshrc.read_text().splitlines():
                if "MIMO_API_KEY" in line and "export" in line:
                    api_key = line.split("=")[-1].strip().strip('"').strip("'")
                    os.environ["MIMO_API_KEY"] = api_key
                    break
    if not api_key:
        print("❌ MIMO_API_KEY not found!")
        print("   Please register at https://platform.xiaomimimo.com/")
        print("   Then set: export MIMO_API_KEY=your_key")
        sys.exit(1)
    return OpenAI(api_key=api_key, base_url=API_BASE)


# =============================================================================
# Test 1: TTS - English Quality & Style Control
# =============================================================================

TTS_TEST_CASES = [
    {
        "name": "basic_greeting",
        "model": "mimo-v2-tts",
        "voice": "default_en",
        "style": None,
        "text": "Hello! Welcome to your English speaking practice. My name is MiMo. Let's have a fun conversation today!",
    },
    {
        "name": "enthusiastic_teacher",
        "model": "mimo-v2.5-tts",
        "voice": "Chloe",
        "style": "Warm, enthusiastic elementary school teacher tone. Speak clearly and slowly with lots of encouragement, like you're talking to a 10-year-old student.",
        "text": "Great job! Your pronunciation is getting better and better. Now let's try saying it one more time, okay?",
    },
    {
        "name": "gentle_correction",
        "model": "mimo-v2.5-tts",
        "voice": "Chloe",
        "style": "Gentle, patient, and encouraging. Like a caring big sister helping her younger sibling. Soft voice, warm and supportive.",
        "text": "That's close! Instead of 'I go to school yesterday,' try saying 'I went to school yesterday.' Can you try that?",
    },
    {
        "name": "excited_storytelling",
        "model": "mimo-v2.5-tts",
        "voice": "Chloe",
        "style": "Excited, bouncy, slightly sing-song tone — like you're bursting with good news. Fast pace, rising pitch at the end of sentences.",
        "text": "Guess what? We're going on an adventure today! We're going to visit a magical restaurant where animals are the chefs!",
    },
    {
        "name": "emotional_empathy",
        "model": "mimo-v2.5-tts",
        "voice": "Chloe",
        "style": "Deeply affectionate, speaking slowly, almost whispering. Very gentle and comforting, like hugging someone with your voice.",
        "text": "It's okay to make mistakes. That's how we learn. I'm here with you, and I'm so proud of you for trying.",
    },
    {
        "name": "text_design_voice",
        "model": "mimo-v2.5-tts-voicedesign",
        "voice": None,
        "style": "Give me a young friendly female English teacher voice, warm and patient, perfect for teaching kids.",
        "text": "Now repeat after me: 'The quick brown fox jumps over the lazy dog.' Very good!",
    },
]


def test_tts(client: OpenAI) -> None:
    """Test MiMo TTS with various English styles."""
    print("=" * 70)
    print("🎙️  TEST 1: MiMo TTS - English Quality & Style Control")
    print("=" * 70)

    for case in TTS_TEST_CASES:
        print(f"\n▶️  Generating: {case['name']}")
        print(f"   Model: {case['model']}")
        if case['style']:
            print(f"   Style: {case['style'][:60]}...")
        print(f"   Text: {case['text'][:60]}...")

        try:
            messages = []
            if case["style"] and case["model"] != "mimo-v2.5-tts-voicedesign":
                # For v2.5-tts, style goes in user message, text in assistant
                messages.append({"role": "user", "content": case["style"]})
                messages.append({"role": "assistant", "content": case["text"]})
            elif case["model"] == "mimo-v2.5-tts-voicedesign":
                # For voice design, style describes the voice, assistant is the text
                messages.append({"role": "user", "content": case["style"]})
                messages.append({"role": "assistant", "content": case["text"]})
            else:
                # For v2-tts, just the text
                messages.append({"role": "assistant", "content": case["text"]})

            audio_config = {"format": "wav"}
            if case["voice"]:
                audio_config["voice"] = case["voice"]

            start = time.time()
            completion = client.chat.completions.create(
                model=case["model"],
                messages=messages,
                audio=audio_config,
            )
            elapsed = time.time() - start

            # Save audio
            audio_data = completion.choices[0].message.audio.data
            audio_bytes = base64.b64decode(audio_data)
            output_path = OUTPUT_DIR / f"{case['name']}.wav"
            output_path.write_bytes(audio_bytes)

            print(f"   ✅ Saved ({len(audio_bytes)} bytes, {elapsed:.2f}s) -> {output_path}")

        except Exception as e:
            print(f"   ❌ Error: {e}")

    print(f"\n📁 All TTS outputs saved to: {OUTPUT_DIR}")


# =============================================================================
# Test 2: Audio Understanding - English Speech Recognition
# =============================================================================

STT_TEST_PROMPTS = [
    "Transcribe the English speech in this audio exactly word for word.",
    "What does the speaker say in this audio? Please provide the exact English transcript.",
    "Listen to this audio and write down every word the speaker says in English.",
]


def test_audio_understanding(client: OpenAI) -> None:
    """Test MiMo Audio Understanding as STT replacement."""
    print("\n" + "=" * 70)
    print("🎧  TEST 2: MiMo Audio Understanding - English STT Quality")
    print("=" * 70)

    # Use a known public English audio sample
    test_audio_url = "https://github.com/soniox/soniox_examples/raw/refs/heads/master/speech_to_text/assets/coffee_shop.mp3"

    print(f"\n📝 Testing with sample audio: {test_audio_url}")
    print("   (A known sample about ordering coffee)")

    for i, prompt in enumerate(STT_TEST_PROMPTS, 1):
        print(f"\n▶️  Prompt variant {i}: {prompt[:50]}...")
        try:
            start = time.time()
            completion = client.chat.completions.create(
                model="mimo-v2.5",
                messages=[
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_audio",
                                "input_audio": {"data": test_audio_url},
                            },
                            {"type": "text", "text": prompt},
                        ],
                    }
                ],
                max_completion_tokens=512,
            )
            elapsed = time.time() - start

            result = completion.choices[0].message.content
            print(f"   ✅ Result ({elapsed:.2f}s):")
            print(f"      {result}")

        except Exception as e:
            print(f"   ❌ Error: {e}")

    # Also test with a simple known phrase for accuracy verification
    print("\n" + "-" * 50)
    print("🎯 Accuracy Test: Simple known phrase")
    print("   We'll generate a TTS audio with known text, then transcribe it back")

    known_text = "The quick brown fox jumps over the lazy dog."
    print(f"   Original: '{known_text}'")

    try:
        # Generate audio
        gen = client.chat.completions.create(
            model="mimo-v2-tts",
            messages=[{"role": "assistant", "content": known_text}],
            audio={"format": "wav", "voice": "default_en"},
        )
        audio_b64 = gen.choices[0].message.audio.data
        audio_bytes = base64.b64decode(audio_b64)

        # Save and get URL (for this test we use base64 directly)
        temp_wav = OUTPUT_DIR / "accuracy_test.wav"
        temp_wav.write_bytes(audio_bytes)

        # Transcribe back using base64
        audio_b64_with_prefix = f"data:audio/wav;base64,{audio_b64}"

        completion = client.chat.completions.create(
            model="mimo-v2.5",
            messages=[
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_audio",
                            "input_audio": {"data": audio_b64_with_prefix},
                        },
                        {
                            "type": "text",
                            "text": "Transcribe the English speech word for word.",
                        },
                    ],
                }
            ],
            max_completion_tokens=256,
        )

        transcribed = completion.choices[0].message.content
        print(f"   Transcribed: '{transcribed}'")

        # Simple accuracy check
        original_lower = known_text.lower().strip(".")
        transcribed_lower = transcribed.lower().strip(".") if transcribed else ""
        if original_lower == transcribed_lower:
            print("   ✅ PERFECT MATCH!")
        else:
            print("   ⚠️  Mismatch - compare manually")

    except Exception as e:
        print(f"   ❌ Error: {e}")


# =============================================================================
# Test 3: End-to-End Conversation
# =============================================================================

SCENARIO_PROMPT = """You are a friendly English teacher named MiMo helping a Chinese primary school student practice speaking English.

Rules:
- Speak in simple, clear English suitable for ages 8-12
- Be encouraging and patient
- Ask ONE question at a time
- When the student makes a mistake, gently correct them
- Keep responses short (1-3 sentences)
- Current scenario: Introducing yourself and talking about hobbies
"""


def test_conversation(client: OpenAI) -> None:
    """Test end-to-end: TTS question -> (simulated student audio) -> Audio Understanding -> Response -> TTS."""
    print("\n" + "=" * 70)
    print("💬  TEST 3: End-to-End Conversation")
    print("=" * 70)

    # Turn 1: AI asks a question
    ai_question = "Hi there! I'm MiMo. What's your name?"
    print(f"\n🤖 AI: {ai_question}")

    try:
        # TTS
        completion = client.chat.completions.create(
            model="mimo-v2-tts",
            messages=[{"role": "assistant", "content": ai_question}],
            audio={"format": "wav", "voice": "default_en"},
        )
        audio_b64 = completion.choices[0].message.audio.data
        audio_bytes = base64.b64decode(audio_b64)

        turn1_path = OUTPUT_DIR / "conversation_turn1_ai.wav"
        turn1_path.write_bytes(audio_bytes)
        print(f"   🔊 Audio saved -> {turn1_path}")

        # Simulate student response with TTS (in real test, this would be recorded)
        student_response = "My name is Tom. I like playing basketball."
        print(f"\n👤 Student (simulated): {student_response}")

        student_tts = client.chat.completions.create(
            model="mimo-v2-tts",
            messages=[{"role": "assistant", "content": student_response}],
            audio={"format": "wav", "voice": "default_en"},
        )
        student_audio_b64 = student_tts.choices[0].message.audio.data

        # AI understands student audio + generates response
        print("\n🧠 AI processing student response...")
        completion = client.chat.completions.create(
            model="mimo-v2.5",
            messages=[
                {"role": "system", "content": SCENARIO_PROMPT},
                {"role": "assistant", "content": ai_question},
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_audio",
                            "input_audio": {
                                "data": f"data:audio/wav;base64,{student_audio_b64}"
                            },
                        },
                        {
                            "type": "text",
                            "text": "The student responded with this audio. Please transcribe and respond naturally as the English teacher.",
                        },
                    ],
                },
            ],
            max_completion_tokens=256,
        )

        ai_response = completion.choices[0].message.content
        print(f"\n🤖 AI: {ai_response}")

        # TTS the response
        response_tts = client.chat.completions.create(
            model="mimo-v2-tts",
            messages=[{"role": "assistant", "content": ai_response}],
            audio={"format": "wav", "voice": "default_en"},
        )
        response_audio = base64.b64decode(response_tts.choices[0].message.audio.data)
        turn2_path = OUTPUT_DIR / "conversation_turn2_ai.wav"
        turn2_path.write_bytes(response_audio)
        print(f"   🔊 Response audio saved -> {turn2_path}")

    except Exception as e:
        print(f"   ❌ Error: {e}")


# =============================================================================
# Main
# =============================================================================

def main():
    print("\n" + "🎓" * 35)
    print("  MiMo Quick Prototype - English Speaking Practice Validation")
    print("🎓" * 35 + "\n")

    client = get_client()
    print(f"✅ MiMo client initialized (base_url: {API_BASE})")

    # Run tests
    test_tts(client)
    test_audio_understanding(client)
    test_conversation(client)

    print("\n" + "=" * 70)
    print("🏁 All tests completed!")
    print(f"📁 Check outputs in: {OUTPUT_DIR}")
    print("=" * 70)

    # Print play command
    print("\n🎵 To play audio files:")
    print(f"   afplay {OUTPUT_DIR}/*.wav    # macOS")
    print(f"   aplay {OUTPUT_DIR}/*.wav     # Linux")


if __name__ == "__main__":
    main()
