#!/usr/bin/env python3
"""
分析用户录音 —— 用 MiMo 转录 + 生成回应 + TTS
===============================================
用法:
    python analyze_recording.py /path/to/recording.m4a
"""

from __future__ import annotations

import base64
import os
import sys
import time
from pathlib import Path

from openai import OpenAI

API_BASE = "https://api.xiaomimimo.com/v1"
OUTPUT_DIR = Path(__file__).parent / "mimo_outputs"
OUTPUT_DIR.mkdir(exist_ok=True)


def get_client() -> OpenAI:
    api_key = os.environ.get("MIMO_API_KEY")
    if not api_key:
        zshrc = Path.home() / ".zshrc"
        if zshrc.exists():
            for line in zshrc.read_text().splitlines():
                if "MIMO_API_KEY" in line and "export" in line:
                    api_key = line.split("=")[-1].strip().strip('"').strip("'")
                    os.environ["MIMO_API_KEY"] = api_key
                    break
    if not api_key:
        print("❌ MIMO_API_KEY not found!")
        sys.exit(1)
    return OpenAI(api_key=api_key, base_url=API_BASE)


def file_to_base64(path: str) -> str:
    """Convert any audio file to base64 data URI with correct MIME type."""
    p = Path(path)
    data = p.read_bytes()
    b64 = base64.b64encode(data).decode("utf-8")

    ext = p.suffix.lower()
    mime_map = {
        ".wav": "audio/wav",
        ".mp3": "audio/mpeg",
        ".m4a": "audio/mp4",
        ".flac": "audio/flac",
        ".ogg": "audio/ogg",
    }
    mime = mime_map.get(ext, "audio/wav")
    return f"data:{mime};base64,{b64}"


def transcribe(client: OpenAI, audio_b64: str) -> str:
    """Transcribe audio with MiMo."""
    completion = client.chat.completions.create(
        model="mimo-v2.5",
        messages=[
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_audio",
                        "input_audio": {"data": audio_b64},
                    },
                    {
                        "type": "text",
                        "text": (
                            "Transcribe the English speech in this audio "
                            "word for word. Only output the exact transcript, nothing else."
                        ),
                    },
                ],
            }
        ],
        max_completion_tokens=256,
    )
    return completion.choices[0].message.content.strip()


def generate_response(client: OpenAI, student_text: str) -> str:
    """Generate AI teacher response."""
    system_prompt = (
        "You are a friendly English teacher named MiMo helping a Chinese primary school student. "
        "Speak in simple, encouraging English. Ask ONE short follow-up question. "
        "If they made grammar mistakes, gently correct them."
    )

    completion = client.chat.completions.create(
        model="mimo-v2.5",
        messages=[
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": f"The student said: '{student_text}'"},
        ],
        max_completion_tokens=150,
    )
    return completion.choices[0].message.content.strip()


def text_to_speech(client: OpenAI, text: str, output_path: str) -> None:
    """Generate enthusiastic teacher TTS."""
    style = (
        "Warm, enthusiastic elementary school teacher tone. "
        "Speak clearly and slowly with lots of encouragement."
    )
    completion = client.chat.completions.create(
        model="mimo-v2.5-tts",
        messages=[
            {"role": "user", "content": style},
            {"role": "assistant", "content": text},
        ],
        audio={"format": "wav", "voice": "Chloe"},
    )
    audio_bytes = base64.b64decode(completion.choices[0].message.audio.data)
    Path(output_path).write_bytes(audio_bytes)


def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze_recording.py <path_to_audio>")
        sys.exit(1)

    audio_path = sys.argv[1]
    if not Path(audio_path).exists():
        print(f"❌ File not found: {audio_path}")
        sys.exit(1)

    print("=" * 60)
    print("🎓 MiMo 录音分析")
    print("=" * 60)
    print(f"\n📁 File: {audio_path}")
    print(f"   Size: {Path(audio_path).stat().st_size / 1024:.1f} KB")

    client = get_client()

    # Step 1: Transcribe
    print("\n🧠 Step 1: Transcribing with MiMo Audio Understanding...")
    audio_b64 = file_to_base64(audio_path)
    start = time.time()
    transcript = transcribe(client, audio_b64)
    elapsed = time.time() - start
    print(f"   ⏱️  {elapsed:.2f}s")
    print(f"\n📝 Transcript:\n   \"{transcript}\"")

    # Step 2: Generate response
    print("\n💬 Step 2: Generating AI teacher response...")
    ai_text = generate_response(client, transcript)
    print(f"\n🤖 AI Response:\n   \"{ai_text}\"")

    # Step 3: TTS
    print("\n🔊 Step 3: Generating TTS (enthusiastic teacher style)...")
    output_path = str(OUTPUT_DIR / "ai_response_to_recording.wav")
    text_to_speech(client, ai_text, output_path)
    print(f"   ✅ Saved -> {output_path}")

    # Summary
    print("\n" + "=" * 60)
    print("🏁 Done!")
    print("=" * 60)
    print(f"\n📊 Results Summary:")
    print(f"   Your recording:     {audio_path}")
    print(f"   MiMo transcript:    '{transcript}'")
    print(f"   AI response:        '{ai_text}'")
    print(f"   AI audio:           {output_path}")
    print("\n🎵 Play AI response:")
    print(f"   afplay '{output_path}'")

    # Ask for feedback
    print("\n💡 Questions for you:")
    print("   1. How accurate was the transcript? Any words missed or wrong?")
    print("   2. Was the AI response natural and appropriate?")
    print("   3. How does the TTS quality compare to the previous tests?")


if __name__ == "__main__":
    main()
