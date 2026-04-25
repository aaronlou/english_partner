//! Async audio capture and playback for duplex streaming.
//!
//! `AudioCapture` bridges cpal's synchronous callback into an async
//! `tokio::sync::mpsc` channel, resampling to 16 kHz mono i16 on the fly.
//!
//! `AudioPlayback` wraps `rodio::Sink` so TTS output can be started and
//! interrupted (stopped) at any time.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ep_core::speech::{AudioChunk, SAMPLE_RATE};
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::trace;

/// Continuous microphone capture.
pub struct AudioCapture {
    _stream: cpal::Stream,
    rx: mpsc::UnboundedReceiver<AudioChunk>,
}

impl AudioCapture {
    /// Start capturing from the default input device.
    pub fn new() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device available"))?;

        let config = device.default_input_config()?;
        let in_sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        println!(
            "🎙️  Audio capture device: {} ({} Hz, {} ch)",
            device.name()?,
            in_sample_rate,
            channels
        );

        let (tx, rx) = mpsc::unbounded_channel::<AudioChunk>();
        let cb_count = Arc::new(AtomicUsize::new(0));
        let cb_count_clone = cb_count.clone();

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let cnt = cb_count_clone.fetch_add(1, Ordering::Relaxed);
                if cnt.is_multiple_of(1000) {
                    let energy = if data.is_empty() {
                        0.0
                    } else {
                        let sum_sq: f32 = data.iter().map(|s| s * s).sum();
                        (sum_sq / data.len() as f32).sqrt()
                    };
                    if energy > 0.001 {
                        trace!(
                            "[cpal callback #{} | samples={} | rms={:.4}]",
                            cnt,
                            data.len(),
                            energy
                        );
                    }
                }

                // Mix to mono
                let mut mono = Vec::with_capacity(data.len() / channels);
                for frame in data.chunks(channels) {
                    let sum: f32 = frame.iter().sum();
                    mono.push(sum / channels as f32);
                }

                // Resample to 16 kHz if needed
                let resampled = if in_sample_rate != SAMPLE_RATE {
                    resample_linear(&mono, in_sample_rate as usize, SAMPLE_RATE as usize)
                } else {
                    mono
                };

                // Convert f32 [-1,1] -> i16
                let samples: Vec<i16> = resampled
                    .iter()
                    .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                    .collect();

                let chunk = AudioChunk {
                    samples,
                    timestamp_ms: 0, // TODO: real timestamps if needed
                };
                let _ = tx.send(chunk);
            },
            |err| eprintln!("❌ Audio capture error: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(Self { _stream: stream, rx })
    }

    /// Receive the next audio chunk.
    pub async fn recv(&mut self) -> Option<AudioChunk> {
        self.rx.recv().await
    }
}

/// Interruptible audio playback.
pub struct AudioPlayback {
    _stream: OutputStream,
    sink: Sink,
}

impl AudioPlayback {
    /// Create a new playback handle.
    pub fn new() -> anyhow::Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        Ok(Self { _stream, sink })
    }

    /// Queue WAV/OGG/MP3 audio data for playback.
    pub fn play_wav(&self, wav_bytes: &[u8]) -> anyhow::Result<()> {
        let cursor = Cursor::new(wav_bytes.to_vec());
        let source = Decoder::new(cursor)?;
        self.sink.append(source);
        Ok(())
    }

    /// Queue raw PCM (mono i16) audio data for playback.
    pub fn play_pcm(&self, samples: &[i16], sample_rate: u32) {
        let samples_f32: Vec<f32> = samples
            .iter()
            .map(|&s| s as f32 / i16::MAX as f32)
            .collect();
        let source = rodio::buffer::SamplesBuffer::new(1, sample_rate, samples_f32);
        self.sink.append(source);
    }

    /// Stop playback immediately and clear the queue.
    pub fn stop(&self) {
        self.sink.stop();
    }

    /// Returns `true` if there are items still in the playback queue.
    ///
    /// Note: this becomes `true` shortly after the last sample finishes.
    pub fn is_playing(&self) -> bool {
        !self.sink.empty()
    }
}

/// Simple linear resampler.
fn resample_linear(input: &[f32], from_rate: usize, to_rate: usize) -> Vec<f32> {
    if from_rate == to_rate {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (input.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let src_idx = i as f64 * ratio;
        let idx_floor = src_idx.floor() as usize;
        let idx_ceil = (idx_floor + 1).min(input.len() - 1);
        let frac = src_idx - idx_floor as f64;
        let val =
            input[idx_floor] as f64 * (1.0 - frac) + input[idx_ceil] as f64 * frac;
        output.push(val as f32);
    }
    output
}
