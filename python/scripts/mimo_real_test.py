#!/usr/bin/env python3
"""
MiMo 真实场景测试 —— 用真人录音验证 STT + TTS
================================================

测试内容：
1. 你用自己的声音录一段英文（带真实口音、语速、停顿）
2. MiMo Audio Understanding 转录
3. 对比原文和转录结果
4. AI 根据转录内容生成回应（TTS）

用法：
    python mimo_real_test.py
"""

from __future__ import annotations

import base64
import os
import subprocess
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


def record_audio(output_path: str, duration_sec: int = 8) -> None:
    """Record audio from microphone."""
    print(f"\n🎙️  Recording for {duration_sec} seconds...")
    print("   Please read the sentence below OUT LOUD:")
    print("   ───────────────────────────────────────")
    print('   "My name is [Your Name]. I am ten years old.')
    print('    I like playing basketball and reading books."')
    print("   ───────────────────────────────────────")
    print(f"   ⏱️  Starting in 2 seconds...")
    time.sleep(2)
    print("   🔴 RECORDING NOW! Speak naturally...")

    if sys.platform == "darwin":
        cmd = [
            "ffmpeg", "-y", "-f", "avfoundation", "-i", ":0",
            "-ar", "16000", "-ac", "1", "-t", str(duration_sec),
            output_path,
        ]
    else:
        cmd = [
            "arecord", "-D", "plughw:1,0", "-d", str(duration_sec),
            "-f", "S16_LE", "-r", "16000", "-c", "1", output_path,
        ]

    subprocess.run(cmd, capture_output=True, check=True)
    print(f"   ✅ Saved to {output_path}")


def audio_to_base64(path: str) -> str:
    """Convert audio file to base64 data URI."""
    data = Path(path).read_bytes()
    b64 = base64.b64encode(data).decode("utf-8")
    return f"data:audio/wav;base64,{b64}"


def transcribe_audio(client: OpenAI, audio_path: str) -> str:
    """Use MiMo Audio Understanding to transcribe."""
    audio_b64 = audio_to_base64(audio_path)

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


def text_to_speech(client: OpenAI, text: str, output_path: str, style: str | None = None) -> None:
    """Generate TTS with enthusiastic teacher style."""
    messages = []
    if style:
        messages.append({"role": "user", "content": style})
    messages.append({"role": "assistant", "content": text})

    completion = client.chat.completions.create(
        model="mimo-v2.5-tts",
        messages=messages,
        audio={"format": "wav", "voice": "Chloe"},
    )
    audio_bytes = base64.b64decode(completion.choices[0].message.audio.data)
    Path(output_path).write_bytes(audio_bytes)


def main():
    print("=" * 60)
    print("🎓 MiMo 真实场景测试 —— 真人录音验证")
    print("=" * 60)

    client = get_client()

    # Step 1: Record
    recording_path = str(OUTPUT_DIR / "my_recording.wav")
    try:
        record_audio(recording_path, duration_sec=8)
    except FileNotFoundError:
        print("\n❌ ffmpeg not found. Install with: brew install ffmpeg")
        print("   Or manually record an 8-second WAV (16kHz mono) and save to:")
        print(f"   {recording_path}")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Recording failed: {e}")
        sys.exit(1)

    # Step 2: Transcribe
    print("\n🧠 Transcribing your recording with MiMo...")
    start = time.time()
    transcript = transcribe_audio(client, recording_path)
    elapsed = time.time() - start
    print(f"   ⏱️  {elapsed:.2f}s")
    print(f"\n📝 Transcript: '{transcript}'")

    # Step 3: Generate AI response
    print("\n💬 Generating AI teacher response...")
    ai_text = generate_response(client, transcript)
    print(f"\n🤖 AI: {ai_text}")

    # Step 4: TTS with enthusiastic style
    print("\n🔊 Generating TTS response...")
    style = (
        "Warm, enthusiastic elementary school teacher tone. "
        "Speak clearly and slowly with lots of encouragement."
    )
    response_path = str(OUTPUT_DIR / "ai_response_to_me.wav")
    text_to_speech(client, ai_text, response_path, style=style)
    print(f"   ✅ Saved -> {response_path}")

    # Summary
    print("\n" + "=" * 60)
    print("🏁 Test Complete!")
    print("=" * 60)
    print("\n📊 Results:")
    print(f"   Your recording:     {recording_path}")
    print(f"   Transcript:         '{transcript}'")
    print(f"   AI response:        '{ai_text}'")
    print(f"   AI response audio:  {response_path}")
    print("\n🎵 Play your recording:")
    print(f"   afplay {recording_path}")
    print("\n🎵 Play AI response:")
    print(f"   afplay {response_path}")
    print("\n💡 How accurate was the transcript? Did MiMo understand your English correctly?")


if __name__ == "__main__":
    main()
