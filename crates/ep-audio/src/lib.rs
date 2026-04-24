//! Audio capture and playback abstraction.
//!
//! Design goals:
//! - 16kHz mono 16-bit PCM (optimal for speech APIs)
//! - Async stream-based API
//! - Minimal latency, zero-copy where possible

pub mod capture;
pub mod playback;
pub mod format;

pub use capture::*;
pub use playback::*;
pub use format::*;
