//! Audio capture and playback abstraction for English Partner.
//!
//! macOS implementation using cpal (CoreAudio) and rodio.

pub mod capture;
pub mod playback;
pub mod format;
pub mod pipeline;
pub mod stream;

pub use capture::*;
pub use playback::*;
pub use format::*;
pub use pipeline::*;
pub use stream::*;
