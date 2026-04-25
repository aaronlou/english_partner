#!/usr/bin/env python3
"""
English Partner - Voice Chat Client
语音交互客户端：录音 → AI 理解 → 语音回复
"""

import base64
import io
import sys
import tempfile
import wave
from pathlib import Path

import httpx
import sounddevice as sd
import numpy as np

BASE_URL = "http://127.0.0.1:8000"
SAMPLE_RATE = 16000
CHANNELS = 1
RECORD_SECONDS = 5  # 默认录音时长


def record_audio(duration: int = RECORD_SECONDS) -> bytes:
    """录制麦克风音频，返回 WAV bytes."""
    print(f"  🎙️  录音中... ({duration}秒，请说话)")
    frames = int(duration * SAMPLE_RATE)
    recording = sd.rec(frames, samplerate=SAMPLE_RATE, channels=CHANNELS, dtype='int16')
    sd.wait()

    # 包装为 WAV
    buffer = io.BytesIO()
    with wave.open(buffer, 'wb') as wf:
        wf.setnchannels(CHANNELS)
        wf.setsampwidth(2)
        wf.setframerate(SAMPLE_RATE)
        wf.writeframes(recording.tobytes())
    return buffer.getvalue()


def play_audio(audio_b64: str) -> None:
    """播放 base64 编码的 WAV 音频."""
    audio_bytes = base64.b64decode(audio_b64)
    tmp_path = Path(tempfile.gettempdir()) / "ep_response.wav"
    tmp_path.write_bytes(audio_bytes)

    import subprocess
    for cmd in [["afplay", str(tmp_path)], ["ffplay", "-nodisp", "-autoexit", str(tmp_path)]]:
        try:
            subprocess.run(cmd, check=True, capture_output=True)
            return
        except Exception:
            continue
    print(f"  ⚠️  无法播放音频，已保存到 {tmp_path}")


def choose_scenario() -> str:
    try:
        resp = httpx.get(f"{BASE_URL}/scenarios", timeout=10)
        resp.raise_for_status()
        scenarios = resp.json()["scenarios"]
    except Exception as exc:
        print(f"无法获取场景列表: {exc}，使用默认场景 restaurant")
        return "restaurant"

    print("\n📚 可选场景:")
    for i, s in enumerate(scenarios, 1):
        print(f"  {i}. {s['name']}")
    while True:
        choice = input("\n请选择场景编号 (默认 1): ").strip()
        if not choice:
            return scenarios[0]["id"]
        try:
            idx = int(choice) - 1
            if 0 <= idx < len(scenarios):
                return scenarios[idx]["id"]
        except ValueError:
            pass
        print("无效选择，请重试")


def main() -> None:
    print("=" * 50)
    print("🤖 English Partner - 语音英语口语练习")
    print("=" * 50)

    # Health check
    try:
        resp = httpx.get(f"{BASE_URL}/health", timeout=5)
        data = resp.json()
        print(f"\n✅ 服务状态: {data['status']} | 提供商: {data['provider']}")
    except Exception as exc:
        print(f"\n❌ 无法连接到 AI 服务 ({BASE_URL}): {exc}")
        print("请先运行: source .venv/bin/activate && python -m ep_ai.main")
        sys.exit(1)

    scenario_id = choose_scenario()

    # 开始对话
    print(f"\n▶️  正在启动场景: {scenario_id} ...")
    resp = httpx.post(
        f"{BASE_URL}/conversation/start",
        data={"scenario_id": scenario_id},
        timeout=30,
    )
    resp.raise_for_status()
    data = resp.json()
    session_id = data["session_id"]

    print(f"\n🎭 场景: {data['scenario_name']}")
    print(f"💬 AI: {data['ai_text']}")
    play_audio(data["audio_base64"])

    print("\n" + "-" * 40)
    print("使用说明:")
    print("  按 Enter 开始录音 (默认5秒)")
    print("  输入数字可调整录音秒数 (如: 3)")
    print("  输入 'q' 退出")
    print("-" * 40 + "\n")

    while True:
        try:
            user_input = input("🎙️  按 Enter 说话 (或输入秒数/q): ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n再见!")
            break

        if user_input.lower() in ("q", "quit", "exit", "退出"):
            print("再见!")
            break

        duration = RECORD_SECONDS
        if user_input.isdigit():
            duration = int(user_input)
            print(f"  ⏱️  录音时长设置为 {duration} 秒")

        # 录音
        audio_bytes = record_audio(duration)
        print(f"  📤 发送音频 ({len(audio_bytes)} bytes) ...")

        # 发送给 AI
        try:
            resp = httpx.post(
                f"{BASE_URL}/conversation/turn",
                data={"session_id": session_id},
                files={"audio": ("voice.wav", audio_bytes, "audio/wav")},
                timeout=120,
            )
            resp.raise_for_status()
        except Exception as exc:
            print(f"  ❌ 请求失败: {exc}")
            continue

        data = resp.json()
        print(f"  📝 识别结果: {data['transcript']}")
        print(f"\n🤖 AI: {data['ai_text']}")
        play_audio(data["audio_base64"])
        print()


if __name__ == "__main__":
    main()
