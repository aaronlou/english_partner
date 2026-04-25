#!/usr/bin/env python3
"""
English Partner - CLI Chat Client
简单的文本交互客户端，连接本地 FastAPI AI 服务。
"""

import base64
import sys
import tempfile
from pathlib import Path

import httpx

BASE_URL = "http://127.0.0.1:8000"


def play_audio(audio_b64: str) -> None:
    """Save and attempt to play audio."""
    audio_bytes = base64.b64decode(audio_b64)
    tmp_path = Path(tempfile.gettempdir()) / "ep_response.wav"
    tmp_path.write_bytes(audio_bytes)
    print(f"  🎵 音频已保存: {tmp_path}")

    # Try common macOS players
    import subprocess
    for cmd in [["afplay", str(tmp_path)], ["play", str(tmp_path)]]:
        try:
            subprocess.run(cmd, check=True, capture_output=True)
            return
        except Exception:
            continue


def choose_scenario() -> str:
    """Fetch and let user pick a scenario."""
    try:
        resp = httpx.get(f"{BASE_URL}/scenarios", timeout=10)
        resp.raise_for_status()
        scenarios = resp.json()["scenarios"]
    except Exception as exc:
        print(f"无法获取场景列表: {exc}，使用默认场景 restaurant")
        return "restaurant"

    print("\n📚 可选场景:")
    for i, s in enumerate(scenarios, 1):
        print(f"  {i}. {s['name']} - {s.get('description', '')}")

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
    print("🤖 English Partner - AI 英语口语练习")
    print("=" * 50)

    # Health check
    try:
        resp = httpx.get(f"{BASE_URL}/health", timeout=5)
        data = resp.json()
        print(f"\n✅ 服务状态: {data['status']} | 提供商: {data['provider']}")
    except Exception as exc:
        print(f"\n❌ 无法连接到 AI 服务 ({BASE_URL}): {exc}")
        print("请先运行: python -m ep_ai.main")
        sys.exit(1)

    scenario_id = choose_scenario()

    print(f"\n▶️  正在启动场景: {scenario_id} ...")
    resp = httpx.post(
        f"{BASE_URL}/conversation/start",
        data={"scenario_id": scenario_id},
        timeout=30,
    )
    resp.raise_for_status()
    data = resp.json()

    session_id = data["session_id"]
    ai_text = data["ai_text"]

    print(f"\n🎭 场景: {data['scenario_name']}")
 print(f"💬 AI: {ai_text}")
    play_audio(data["audio_base64"])

    print("\n输入你的回复 (或输入 'quit' 退出):\n")

    while True:
        try:
            user_input = input("👤 你: ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n再见!")
            break

        if not user_input:
            continue
        if user_input.lower() in ("quit", "exit", "q", "退出"):
            print("再见!")
            break

        resp = httpx.post(
            f"{BASE_URL}/conversation/respond",
            data={"session_id": session_id, "student_text": user_input},
            timeout=60,
        )
        resp.raise_for_status()
        data = resp.json()

        print(f"🤖 AI: {data['ai_text']}")
        play_audio(data["audio_base64"])
        print()


if __name__ == "__main__":
    main()
