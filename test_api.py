#!/usr/bin/env python3
"""Quick prototype to test Soniox API capabilities."""

import os
from soniox import SonioxClient
import httpx

api_key = os.environ.get("SONIOX_API_KEY")
if not api_key:
    # Try to load from ~/.zshrc
    with open(os.path.expanduser("~/.zshrc")) as f:
        for line in f:
            if "SONIOX_API_KEY" in line and "export" in line:
                api_key = line.split("=")[-1].strip().strip('"').strip("'")
                break

client = SonioxClient(api_key=api_key)

# 1. List models and voices
print("=" * 60)
print("1. FETCHING MODELS & VOICES")
print("=" * 60)
resp = httpx.get(
    "https://api.soniox.com/v1/models",
    headers={"Authorization": f"Bearer {client.api_key}"},
)
print(f"Models status: {resp.status_code}")
if resp.status_code == 200:
    data = resp.json()
    for m in data.get("models", []):
        print(f"\nModel: {m.get('id')} - {m.get('name')}")
        if "voices" in m:
            for v in m["voices"]:
                langs = v.get("languages", v.get("language", "unknown"))
                print(f"  Voice: {v.get('id'):12s} - {v.get('name'):20s} - lang: {langs}")
        if "languages" in m and isinstance(m["languages"], list):
            print(f"  Supported languages: {m['languages']}")

# 2. Test TTS (English)
print("\n" + "=" * 60)
print("2. TESTING TTS (English)")
print("=" * 60)
try:
    written = client.tts.generate_to_file(
        "hello.wav",
        text="Hello! Welcome to your English speaking practice. Let us talk about your favorite hobby.",
        model="tts-rt-v1-preview",
        language="en",
        voice="Adrian",
        audio_format="wav",
    )
    print(f"TTS success: {written} bytes written to hello.wav")
except Exception as e:
    print(f"TTS error: {e}")

# 3. Test TTS (Chinese)
print("\n" + "=" * 60)
print("3. TESTING TTS (Chinese)")
print("=" * 60)
try:
    written = client.tts.generate_to_file(
        "hello_zh.wav",
        text="你好！欢迎来到英语口语练习。今天我们一起来练习日常对话。",
        model="tts-rt-v1-preview",
        language="zh",
        voice="Adrian",  # Try default voice
        audio_format="wav",
    )
    print(f"TTS Chinese success: {written} bytes written to hello_zh.wav")
except Exception as e:
    print(f"TTS Chinese error: {e}")

# 4. Test async STT with a sample file
print("\n" + "=" * 60)
print("4. TESTING ASYNC STT")
print("=" * 60)
try:
    transcription = client.stt.transcribe(
        audio_url="https://soniox.com/media/examples/coffee_shop.mp3",
        language_hints=["en"],
    )
    print(f"Transcription created: {transcription.id}")
    client.stt.wait(transcription.id)
    transcript = client.stt.get_transcript(transcription.id)
    print(f"Transcript text: {transcript.text[:200]}...")
except Exception as e:
    print(f"STT error: {e}")
