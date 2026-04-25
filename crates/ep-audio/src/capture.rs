//! Microphone audio capture using cpal.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::WavSpec;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Record audio from the default microphone for a fixed duration.
/// Output is 16kHz mono 16-bit PCM WAV.
pub fn record_to_file(path: &Path, duration_secs: u32) -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No default input device available")?;

    // Request 16kHz mono f32 — cpal will give us the closest supported config.
    let supported_config = device.default_input_config()?;
    println!(
        "🎙️  Using input device: {} ({:?})",
        device.name()?,
        supported_config
    );

    let sample_rate = supported_config.sample_rate().0;
    let channels = supported_config.channels() as usize;

    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let samples = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = samples.clone();

    let err_fn = |err| eprintln!("❌ Audio stream error: {}", err);

    let stream = device.build_input_stream(
        &supported_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = samples_clone.lock().unwrap();
            for frame in data.chunks(channels) {
                // Mix down to mono by averaging channels
                let sum: f32 = frame.iter().sum();
                let mono = sum / channels as f32;
                buf.push(mono);
            }
        },
        err_fn,
        None,
    )?;

    stream.play()?;
    println!("   ⏺️  Recording... ({} seconds)", duration_secs);
    std::thread::sleep(std::time::Duration::from_secs(duration_secs as u64));
    drop(stream);

    // Resample to 16kHz if needed and write to WAV
    let raw_samples = samples.lock().unwrap().clone();
    let resampled = if sample_rate != 16000 {
        resample_linear(&raw_samples, sample_rate as usize, 16000)
    } else {
        raw_samples
    };

    let mut writer = hound::WavWriter::create(path, spec)?;
    for s in resampled {
        let clamped = s.clamp(-1.0, 1.0);
        let i16_sample = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(i16_sample)?;
    }
    writer.finalize()?;

    println!("   ✅ Saved to {}", path.display());
    Ok(())
}

/// Record audio until the user presses Enter.
/// Spawns a thread for recording, returns when user confirms.
pub fn record_until_stop(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No default input device available")?;

    let supported_config = device.default_input_config()?;
    println!(
        "🎙️  Using input device: {}",
        device.name()?
    );

    let sample_rate = supported_config.sample_rate().0;
    let channels = supported_config.channels() as usize;

    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let samples = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = samples.clone();

    let err_fn = |err| eprintln!("❌ Audio stream error: {}", err);

    let stream = device.build_input_stream(
        &supported_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = samples_clone.lock().unwrap();
            for frame in data.chunks(channels) {
                let sum: f32 = frame.iter().sum();
                let mono = sum / channels as f32;
                buf.push(mono);
            }
        },
        err_fn,
        None,
    )?;

    stream.play()?;
    println!("   🔴 Recording... Press ENTER when done speaking.");

    let mut dummy = String::new();
    let _ = std::io::stdin().read_line(&mut dummy);

    drop(stream);
    println!("   ⏹️  Stopped recording.");

    let raw_samples = samples.lock().unwrap().clone();
    let resampled = if sample_rate != 16000 {
        resample_linear(&raw_samples, sample_rate as usize, 16000)
    } else {
        raw_samples
    };

    let mut writer = hound::WavWriter::create(path, spec)?;
    for s in resampled {
        let clamped = s.clamp(-1.0, 1.0);
        let i16_sample = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(i16_sample)?;
    }
    writer.finalize()?;

    println!("   ✅ Saved to {}", path.display());
    Ok(())
}

/// Simple linear resampling.
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
        let val = input[idx_floor] as f64 * (1.0 - frac) + input[idx_ceil] as f64 * frac;
        output.push(val as f32);
    }
    output
}
