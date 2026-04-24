#!/usr/bin/env python3
"""
English Partner - MiMo-Only MVP
===============================
纯 MiMo 版本的英语口语练习原型。

功能：
- 选择场景（自我介绍 / 餐厅点餐 / 购物）
- AI 用 enthusiastic teacher 风格提问（MiMo TTS）
- 学生录音回答
- MiMo 理解并生成回应
- 多轮对话

运行：
    cd /Users/lousiyuan/AI_PLAYGROUND/lsy_build/english_partner
    source .venv/bin/activate
    python python/scripts/mvp_mimo_only.py
"""

from __future__ import annotations

import base64
import os
import subprocess
import sys
import time
from pathlib import Path

from openai import OpenAI

# =============================================================================
# Configuration
# =============================================================================

API_BASE = "https://api.xiaomimimo.com/v1"
OUTPUT_DIR = Path(__file__).parent / "mimo_outputs" / "mvp_session"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

SCENARIOS = {
    "1": {
        "name": "自我介绍 (Self Introduction)",
        "system_prompt": (
            "You are a friendly English teacher named MiMo helping a Chinese primary school student practice self-introduction.\n"
            "Rules:\n"
            "- Speak in simple English suitable for ages 8-12\n"
            "- Be warm and encouraging\n"
            "- Ask ONE question at a time\n"
            "- Gently correct mistakes by repeating the correct form\n"
            "- Keep responses to 1-3 sentences\n"
            "- Current topic: introducing name, age, hobbies, and family"
        ),
        "opening": "Hi there! I'm MiMo, your English practice partner. What's your name?",
    },
    "2": {
        "name": "餐厅点餐 (Restaurant Order)",
        "system_prompt": (
            "You are a friendly waiter at a Western restaurant. A Chinese primary school student is practicing ordering food in English.\n"
            "Rules:\n"
            "- Speak clearly and slowly\n"
            "- Use simple vocabulary\n"
            "- Gently correct mistakes\n"
            "- Ask ONE question at a time\n"
            "- Keep responses to 1-3 sentences\n"
            "- Guide them through: greeting → ordering → asking about price → saying thanks"
        ),
        "opening": "Hello! Welcome to our restaurant! What would you like to order today?",
    },
    "3": {
        "name": "购物 (Shopping)",
        "system_prompt": (
            "You are a friendly shop assistant at a toy store. A Chinese primary school student is practicing shopping in English.\n"
            "Rules:\n"
            "- Speak in simple, friendly English\n"
            "- Be patient and encouraging\n"
            "- Ask ONE question at a time\n"
            "- Gently correct mistakes\n"
            "- Keep responses to 1-3 sentences\n"
            "- Guide them through: greeting → asking about items → prices → making a decision"
        ),
        "opening": "Hi! Welcome to our toy store! Are you looking for something special today?",
    },
}

TTS_STYLE = (
    "Warm, enthusiastic elementary school teacher tone. "
    "Speak clearly and slowly with lots of encouragement, "
    "like you're talking to a 10-year-old student. "
    "Use natural intonation and friendly energy."
)

# =============================================================================
# Helpers
# =============================================================================

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
        print("❌ MIMO_API_KEY not found! Please set it in ~/.zshrc")
        sys.exit(1)
    return OpenAI(api_key=api_key, base_url=API_BASE)


def print_banner():
    print("\n" + "=" * 60)
    print("🎓 English Partner - MiMo Only MVP")
    print("   AI-powered English speaking practice for kids")
    print("=" * 60 + "\n")


def choose_scenario() -> dict:
    print("📚 Choose a practice scenario:\n")
    for key, val in SCENARIOS.items():
        print(f"   [{key}] {val['name']}")
    print("   [q] Quit\n")

    while True:
        choice = input("Your choice: ").strip().lower()
        if choice == "q":
            sys.exit(0)
        if choice in SCENARIOS:
            return SCENARIOS[choice]
        print("Invalid choice. Please try again.")


def tts_speak(client: OpenAI, text: str, output_path: str) -> None:
    """Generate enthusiastic teacher TTS."""
    completion = client.chat.completions.create(
        model="mimo-v2.5-tts",
        messages=[
            {"role": "user", "content": TTS_STYLE},
            {"role": "assistant", "content": text},
        ],
        audio={"format": "wav", "voice": "Chloe"},
    )
    audio_bytes = base64.b64decode(completion.choices[0].message.audio.data)
    Path(output_path).write_bytes(audio_bytes)


def play_audio(path: str) -> None:
    """Play audio file."""
    if sys.platform == "darwin":
        subprocess.run(["afplay", path], capture_output=True)
    else:
        subprocess.run(["aplay", path], capture_output=True)


def record_student_audio(output_path: str, duration_sec: int = 8) -> bool:
    """Record audio from microphone."""
    print(f"\n🎙️  Recording for {duration_sec} seconds...")
    print("   🔴 SPEAK NOW! Press Enter when ready to start...")
    input()
    print("   ⏺️  Recording... Speak now!")

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

    try:
        subprocess.run(cmd, capture_output=True, check=True)
        print(f"   ✅ Saved to {output_path}")
        return True
    except FileNotFoundError:
        print("   ❌ ffmpeg not found. Install: brew install ffmpeg")
        return False
    except subprocess.CalledProcessError as e:
        print(f"   ❌ Recording failed: {e}")
        return False


def transcribe_audio(client: OpenAI, audio_path: str) -> str:
    """Transcribe student audio with MiMo."""
    with open(audio_path, "rb") as f:
        audio_b64 = base64.b64encode(f.read()).decode()
    data_uri = f"data:audio/wav;base64,{audio_b64}"

    completion = client.chat.completions.create(
        model="mimo-v2.5",
        messages=[
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_audio",
                        "input_audio": {"data": data_uri},
                    },
                    {
                        "type": "text",
                        "text": (
                            "Transcribe the English speech in this audio. "
                            "If the speaker uses Chinese, include it. "
                            "Output only the transcript."
                        ),
                    },
                ],
            }
        ],
        max_completion_tokens=256,
    )
    return completion.choices[0].message.content.strip()


def generate_ai_response(client: OpenAI, system_prompt: str, history: list[dict], student_text: str) -> str:
    """Generate AI teacher response."""
    messages = [{"role": "system", "content": system_prompt}]
    for h in history:
        if h["role"] == "ai":
            messages.append({"role": "assistant", "content": h["text"]})
        else:
            messages.append({"role": "user", "content": h["text"]})
    messages.append({"role": "user", "content": f"The student said: '{student_text}'"})

    completion = client.chat.completions.create(
        model="mimo-v2.5",
        messages=messages,
        max_completion_tokens=200,
    )
    return completion.choices[0].message.content.strip()


# =============================================================================
# Main Session
# =============================================================================

def run_session(client: OpenAI, scenario: dict) -> None:
    """Run a practice session."""
    print(f"\n🎯 Scenario: {scenario['name']}")
    print("-" * 60)
    print("💡 Tips:")
    print("   • Speak clearly into your microphone")
    print("   • It's okay to make mistakes — that's how we learn!")
    print("   • You can speak in English or Chinese (MiMo understands both)")
    print("   • Type 'quit' at any time to end the session\n")

    history: list[dict] = []
    turn = 0

    # Opening
    ai_text = scenario["opening"]
    print(f"🤖 MiMo: {ai_text}\n")

    wav_path = str(OUTPUT_DIR / f"turn_{turn}_ai.wav")
    tts_speak(client, ai_text, wav_path)
    play_audio(wav_path)

    history.append({"role": "ai", "text": ai_text})

    while True:
        turn += 1
        print(f"\n--- Turn {turn} ---")

        # Student speaks
        student_wav = str(OUTPUT_DIR / f"turn_{turn}_student.wav")
        if not record_student_audio(student_wav, duration_sec=8):
            fallback = input("📝 Recording failed. Type your response instead: ").strip()
            if fallback.lower() == "quit":
                break
            student_text = fallback
        else:
            # Play back student's recording
            print("   ▶️  Playing back your recording...")
            play_audio(student_wav)

            # Transcribe
            print("\n🧠 MiMo is listening...")
            start = time.time()
            student_text = transcribe_audio(client, student_wav)
            elapsed = time.time() - start
            print(f"   ⏱️  {elapsed:.1f}s")
            print(f"   📝 Transcript: \"{student_text}\"")

        if student_text.lower() in ("quit", "exit", "q"):
            break

        # AI responds
        print("\n💬 MiMo is thinking...")
        ai_text = generate_ai_response(client, scenario["system_prompt"], history, student_text)
        print(f"\n🤖 MiMo: {ai_text}\n")

        # TTS + Play
        ai_wav = str(OUTPUT_DIR / f"turn_{turn}_ai.wav")
        tts_speak(client, ai_text, ai_wav)
        play_audio(ai_wav)

        history.append({"role": "student", "text": student_text})
        history.append({"role": "ai", "text": ai_text})

        # Limit history to last 6 turns to save tokens
        if len(history) > 12:
            history = history[-12:]

    # End session
    print("\n" + "=" * 60)
    print("🏁 Session complete!")
    print("=" * 60)
    print(f"\n📁 All recordings saved to: {OUTPUT_DIR}")
    print("\n👋 Goodbye! Keep practicing! 🌟")


def main():
    print_banner()

    # Check ffmpeg
    if subprocess.run(["which", "ffmpeg"], capture_output=True).returncode != 0:
        print("⚠️  ffmpeg not found. Please install it first:")
        print("   brew install ffmpeg")
        sys.exit(1)

    client = get_client()
    print(f"✅ MiMo client ready\n")

    scenario = choose_scenario()
    run_session(client, scenario)


if __name__ == "__main__":
    main()
