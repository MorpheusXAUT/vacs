pub mod config;
mod device;
pub mod input;
pub mod output;
pub mod sources;

pub use device::Device;
pub use device::DeviceType;

use bytes::Bytes;

pub type EncodedAudioFrame = Bytes;

pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME_DURATION_MS: u64 = 20;
const FRAME_SIZE: usize = SAMPLE_RATE as usize * FRAME_DURATION_MS as usize / 1000;
