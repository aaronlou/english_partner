#!/usr/bin/env python3
"""
English Partner - Voice Chat Client with VAD
语音交互客户端（VAD 自动停）：
  按 Enter 开始录音 → 检测到说话结束自动停止 → AI 语音回复
"""

import base64
import io
import collections
import sys
import tempfile
import wave
from pathlib import Path

import httpx
import numpy as np
import sounddevice as sd
import webrtcvad

BASE_URL = "http://127.0.0.1:8000"
SAMPLE_RATE = 16000
CHANNELS = 1
FRAME_DURATION_MS = 30  # 10, 20, or 30
VAD_AGGRESSIVENESS = 2  # 0-3, higher = more strict
SILENCE_FRAMES_THRESHOLD = 20  # ~0.6s of silence


def record_with_vad() -> bytes:
    """录制音频，使用 WebRTC VAD 自动检测说话结束."""
    vad = webrtcvad.Vad(VAD_AGGRESSIVENESS)
    frame_size = int(SAMPLE_RATE * FRAME_DURATION_MS / 1000)
    
    buffer = io.BytesIO()
    # 先写占位 WAV 头
    wf = wave.open(buffer, 'wb')
    wf.setnchannels(CHANNELS)
    wf.setsampwidth(2)
    wf.setframerate(SAMPLE_RATE)

    ring_buffer = collections.deque(maxlen=SILENCE_FRAMES_THRESHOLD)
    triggered = False
    num_voiced = 0
    num_unvoiced = 0
    all_frames = []

    print("  🎙️  请说话... (说完自动停止)")

    import threading
    stop_event = threading.Event()

    def callback(indata, frames, time_info, status):
        nonlocal triggered, num_voiced, num_unvoiced
        if status:
            print(f"  [audio status: {status}]", file=sys.stderr)
        
        pcm = indata[:, 0].astype(np.int16).tobytes()
        
        offset = 0
        while offset + frame_size * 2 <= len(pcm):
            frame = pcm[offset:offset + frame_size * 2]
            offset += frame_size * 2
            
            is_speech = vad.is_speech(frame, SAMPLE_RATE)
            
            if not triggered:
                ring_buffer.append((frame, is_speech))
                num_voiced = sum(1 for _, speech in ring_buffer if speech)
                if num_voiced > SILENCE_FRAMES_THRESHOLD * 0.6:
                    triggered = True
                    for f, _ in ring_buffer:
                        all_frames.append(f)
                    ring_buffer.clear()
                    print("  ✅ 检测到语音，正在聆听...")
            else:
                all_frames.append(frame)
                ring_buffer.append((frame, is_speech))
                num_unvoiced = sum(1 for _, speech in ring_buffer if not speech)
                if num_unvoiced > SILENCE_FRAMES_THRESHOLD * 0.8:
                    stop_event.set()
                    raise sd.CallbackStop()

    with sd.RawInputStream(
        samplerate=SAMPLE_RATE,
        blocksize=frame_size,
        dtype='int16',
        channels=CHANNELS,
        callback=callback,
    ):
        try:
            stop_event.wait(timeout=30)
        except KeyboardInterrupt:
            pass

    if not all_frames:
        print("  ⚠️  未检测到语音")
        return b''

    print(f"  🛑 语音结束，共 {len(all_frames)} 帧")
    wf.writeframes(b''.join(all_frames))
    wf.close()
    return buffer.getvalue()


def play_audio(audio_b64: str) -> None:
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
    print(f"  ⚠️ 无法播放音频，已保存到 {tmp_path}")


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
    print("🤖 English Partner - 语音英语口语练习 (VAD版)")
    print("=" * 50)

    try:
        resp = httpx.get(f"{BASE_URL}/health", timeout=5)
        data = resp.json()
        print(f"\n✅ 服务状态: {data['status']} | 提供商: {data['provider']}")
    except Exception as exc:
        print(f"\n❌ 无法连接到 AI 服务: {exc}")
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

    print(f"\n🎭 场景: {data['scenario_name']}")
    print(f"💬 AI: {data['ai_text']}")
    play_audio(data["audio_base64"])

    print("\n" + "-" * 40)
    print("使用说明:")
    print("  按 Enter → 开始录音 → 说完自动停 → AI 语音回复")
    print("  输入 'q' 退出")
    print("-" * 40 + "\n")

    while True:
        try:
            user_input = input("🎙️  按 Enter 说话: ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n再见!")
            break

        if user_input.lower() in ("q", "quit", "exit"):
            print("再见!")
            break

        audio_bytes = record_with_vad()
        if not audio_bytes:
            print("  没有录到声音，再试一次\n")
            continue

        print(f"  📤 发送音频 ({len(audio_bytes)} bytes) ...")
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
