//! Audio playback (TTS output) using rodio.

use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

/// Play WAV audio data from memory.
pub fn play_wav_bytes(wav_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let cursor = Cursor::new(wav_data.to_vec());
    let source = Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}

/// Play a WAV file from disk.
pub fn play_wav_file(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    play_wav_bytes(&data)
}
