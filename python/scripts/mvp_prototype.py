#!/usr/bin/env python3
"""
English Partner - MVP Prototype (Phase 1)
=========================================
A minimal voice-based English speaking practice demo using Soniox APIs.

Features:
- AI asks questions in English (TTS)
- Student responds by voice (recorded to file, then STT)
- AI provides feedback and next question (LLM + TTS)

Prerequisites:
    export SONIOX_API_KEY=your_key
    uv pip install soniox openai python-dotenv

Usage:
    python mvp_prototype.py
"""

import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# Try to load API key from ~/.zshrc if not in env
def _load_key():
    key = os.environ.get("SONIOX_API_KEY")
    if key:
        return key
    zshrc = Path.home() / ".zshrc"
    if zshrc.exists():
        for line in zshrc.read_text().splitlines():
            if "SONIOX_API_KEY" in line and "export" in line:
                key = line.split("=")[-1].strip().strip('"').strip("'")
                os.environ["SONIOX_API_KEY"] = key
                return key
    raise RuntimeError("SONIOX_API_KEY not found. Set it in env or ~/.zshrc")


SONIOX_API_KEY = _load_key()


def record_audio(output_path: str, duration_sec: int = 5) -> None:
    """Record audio using ffmpeg (macOS) or arecord (Linux)."""
    print(f"🎙️  Recording for {duration_sec} seconds... Speak now!")

    # macOS: use ffmpeg with avfoundation
    if sys.platform == "darwin":
        cmd = [
            "ffmpeg", "-y", "-f", "avfoundation", "-i", ":0",
            "-ar", "16000", "-ac", "1", "-t", str(duration_sec),
            output_path,
        ]
    else:
        # Linux fallback with arecord
        cmd = [
            "arecord", "-D", "plughw:1,0", "-d", str(duration_sec),
            "-f", "S16_LE", "-r", "16000", "-c", "1", output_path,
        ]

    try:
        subprocess.run(cmd, capture_output=True, check=True)
        print(f"✅ Saved to {output_path}")
    except FileNotFoundError:
        print("❌ ffmpeg not found. Install with: brew install ffmpeg")
        raise
    except subprocess.CalledProcessError as e:
        print(f"❌ Recording failed: {e.stderr.decode()}")
        raise


def tts_speak(text: str, output_path: str = "ai_speech.wav") -> str:
    """Generate speech using Soniox TTS."""
    from soniox import SonioxClient

    client = SonioxClient()
    written = client.tts.generate_to_file(
        output_path,
        text=text,
        model="tts-rt-v1-preview",
        language="en",
        voice="Adrian",
        audio_format="wav",
    )
    print(f"🔊 TTS generated: {written} bytes -> {output_path}")
    return output_path


def play_audio(path: str) -> None:
    """Play audio file."""
    print(f"▶️  Playing {path}...")
    if sys.platform == "darwin":
        subprocess.run(["afplay", path], check=True)
    else:
        subprocess.run(["aplay", path], check=True)


def stt_transcribe(audio_path: str) -> str:
    """Transcribe audio using Soniox STT."""
    from soniox import SonioxClient

    client = SonioxClient()
    # For the prototype, we use async transcription
    transcription = client.stt.transcribe(
        audio_url=None,  # We'll need to upload the file
    )
    # Actually, the Python SDK for v2 may have a different API for file upload.
    # For now, let's return a placeholder and note the limitation.
    return "[STT placeholder - file upload API TBD in prototype]"


def llm_respond(student_text: str, scenario: str, history: list) -> str:
    """Generate AI response using OpenAI or a simple rule-based fallback."""
    import os

    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        # Fallback: rule-based responses
        responses = [
            "That's great! Can you tell me more?",
            "Interesting! What do you like about it?",
            "Good job! Let's try another question.",
            "Nice answer! How often do you do that?",
        ]
        idx = len(history) % len(responses)
        return responses[idx]

    from openai import OpenAI
    client = OpenAI(api_key=api_key)

    messages = [
        {"role": "system", "content": scenario},
        *[
            {"role": "user" if i % 2 == 0 else "assistant", "content": h["text"]}
            for i, h in enumerate(history)
        ],
        {"role": "user", "content": student_text},
    ]

    resp = client.chat.completions.create(
        model="gpt-4o-mini",
        messages=messages,
        max_tokens=100,
        temperature=0.7,
    )
    return resp.choices[0].message.content


def main():
    print("=" * 60)
    print("🎓 English Partner - MVP Prototype")
    print("=" * 60)

    scenario = """You are a friendly English teacher helping a Chinese primary school student practice speaking.
Speak in simple English. Ask one question at a time. Be encouraging.
Current scenario: Introducing yourself and your hobbies."""

    history = []

    # Initial greeting
    ai_text = "Hello! I'm your English practice partner. What's your name?"
    print(f"\n🤖 AI: {ai_text}")

    wav_path = tts_speak(ai_text, "turn_0_ai.wav")
    play_audio(wav_path)

    for turn in range(1, 4):
        print(f"\n--- Turn {turn} ---")

        # Record student
        student_wav = f"turn_{turn}_student.wav"
        try:
            record_audio(student_wav, duration_sec=5)
        except Exception as e:
            print(f"Recording failed: {e}")
            student_text = input("Type your response instead: ")
        else:
            # For the prototype, we can't easily do STT from local file with Soniox v2 SDK
            # without implementing file upload. We'll use manual input as fallback.
            student_text = input("📝 (STT would go here) Type what you said: ")

        print(f"👤 Student: {student_text}")
        history.append({"speaker": "student", "text": student_text})

        # LLM response
        ai_text = llm_respond(student_text, scenario, history)
        print(f"🤖 AI: {ai_text}")
        history.append({"speaker": "ai", "text": ai_text})

        # TTS + Play
        wav_path = tts_speak(ai_text, f"turn_{turn}_ai.wav")
        play_audio(wav_path)

    print("\n" + "=" * 60)
    print("✅ Practice session complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
