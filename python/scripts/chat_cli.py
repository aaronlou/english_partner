#!/usr/bin/env python3
"""
English Partner - Interactive CLI Chat Client
=============================================

Calls the FastAPI service for voice/text conversations.

Usage:
    source .venv/bin/activate
    python python/scripts/chat_cli.py
"""

from __future__ import annotations

import base64
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import httpx

API_BASE = "http://127.0.0.1:8000"


def print_banner():
    print("\n" + "=" * 60)
    print("🎓 English Partner - Interactive Practice")
    print("   Talk with MiMo, your AI English teacher!")
    print("=" * 60 + "\n")


def play_audio(wav_bytes: bytes) -> None:
    """Play WAV audio bytes using system player."""
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        f.write(wav_bytes)
        path = f.name
    try:
        if sys.platform == "darwin":
            subprocess.run(["afplay", path], capture_output=True)
        else:
            subprocess.run(["aplay", path], capture_output=True)
    finally:
        Path(path).unlink(missing_ok=True)


def record_audio(duration: int = 8) -> bytes | None:
    """Record audio from microphone. Returns WAV bytes or None."""
    print(f"\n🎙️  Recording for {duration} seconds...")
    input("   Press ENTER to start recording...")
    print("   🔴 Recording... Speak now!")

    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        out_path = f.name

    if sys.platform == "darwin":
        cmd = [
            "ffmpeg", "-y", "-f", "avfoundation", "-i", ":0",
            "-ar", "16000", "-ac", "1", "-t", str(duration),
            out_path,
        ]
    else:
        cmd = [
            "arecord", "-D", "plughw:1,0", "-d", str(duration),
            "-f", "S16_LE", "-r", "16000", "-c", "1", out_path,
        ]

    try:
        subprocess.run(cmd, capture_output=True, check=True)
        wav_bytes = Path(out_path).read_bytes()
        Path(out_path).unlink(missing_ok=True)
        print("   ✅ Recording saved")
        return wav_bytes
    except (FileNotFoundError, subprocess.CalledProcessError):
        Path(out_path).unlink(missing_ok=True)
        return None


def choose_scenario() -> str:
    """List scenarios and let user choose."""
    resp = httpx.get(f"{API_BASE}/scenarios")
    scenarios = resp.json()["scenarios"]

    print("📚 Choose a practice scenario:\n")
    for i, s in enumerate(scenarios, 1):
        print(f"   [{i}] {s['name']}")
    print("   [q] Quit\n")

    while True:
        choice = input("Your choice: ").strip().lower()
        if choice == "q":
            sys.exit(0)
        try:
            idx = int(choice) - 1
            if 0 <= idx < len(scenarios):
                return scenarios[idx]["id"]
        except ValueError:
            pass
        print("   Invalid choice. Try again.")


def start_conversation(scenario_id: str) -> dict:
    """Start a new conversation. Returns session info."""
    resp = httpx.post(f"{API_BASE}/conversation/start", data={"scenario_id": scenario_id})
    resp.raise_for_status()
    return resp.json()


def text_turn(session_id: str, text: str) -> dict:
    """Send a text turn."""
    resp = httpx.post(
        f"{API_BASE}/conversation/respond",
        data={"session_id": session_id, "student_text": text},
    )
    resp.raise_for_status()
    return resp.json()


def audio_turn(session_id: str, wav_bytes: bytes) -> dict:
    """Send an audio turn."""
    resp = httpx.post(
        f"{API_BASE}/conversation/turn",
        data={"session_id": session_id},
        files={"audio": ("student.wav", wav_bytes, "audio/wav")},
    )
    resp.raise_for_status()
    return resp.json()


def main():
    print_banner()

    # Health check
    try:
        health = httpx.get(f"{API_BASE}/health").json()
        print(f"✅ Service ready (provider: {health['provider']})\n")
    except httpx.ConnectError:
        print(f"❌ Cannot connect to {API_BASE}")
        print("   Make sure the FastAPI server is running:")
        print("   source .venv/bin/activate && python -m ep_ai.main")
        sys.exit(1)

    scenario_id = choose_scenario()
    result = start_conversation(scenario_id)

    session_id = result["session_id"]
    ai_text = result["ai_text"]

    print(f"\n🤖 MiMo: {ai_text}\n")
    play_audio(base64.b64decode(result["audio_base64"]))

    turn = 0
    while True:
        turn += 1
        print(f"\n--- Turn {turn} ---")
        print("\n   [1] 🎙️  Speak (record audio)")
        print("   [2] 📝 Type text")
        print("   [3] 👋 Quit")
        choice = input("\nYour choice: ").strip()

        if choice == "3" or choice.lower() in ("q", "quit"):
            break

        if choice == "1":
            wav_bytes = record_audio(duration=8)
            if wav_bytes is None:
                print("   ❌ Recording failed. Falling back to text.")
                text = input("📝 Type your response: ").strip()
                if text.lower() in ("quit", "q"):
                    break
                result = text_turn(session_id, text)
            else:
                print("\n🧠 MiMo is thinking...")
                start = time.time()
                result = audio_turn(session_id, wav_bytes)
                print(f"   ⏱️  {time.time() - start:.1f}s")
        elif choice == "2":
            text = input("📝 Type your response: ").strip()
            if text.lower() in ("quit", "q"):
                break
            print("\n🧠 MiMo is thinking...")
            start = time.time()
            result = text_turn(session_id, text)
            print(f"   ⏱️  {time.time() - start:.1f}s")
        else:
            print("   Invalid choice.")
            continue

        transcript = result.get("transcript", "")
        ai_text = result["ai_text"]

        if transcript:
            print(f"\n   📝 You said: \"{transcript}\"")
        print(f"\n🤖 MiMo: {ai_text}\n")
        play_audio(base64.b64decode(result["audio_base64"]))

    print("\n" + "=" * 60)
    print("🏁 Session complete! Keep practicing! 🌟")
    print("=" * 60 + "\n")


if __name__ == "__main__":
    main()
