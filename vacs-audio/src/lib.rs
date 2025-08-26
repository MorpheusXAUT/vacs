pub mod config;
mod device;
mod dsp;
pub(crate) mod mixer;
pub mod sources;
pub mod stream;

pub use device::Device;
pub use device::DeviceSelector;
pub use device::DeviceType;

use bytes::Bytes;

pub type EncodedAudioFrame = Bytes;

pub const TARGET_SAMPLE_RATE: u32 = 48_000;
pub const FRAME_DURATION_MS: u64 = 20;
const FRAME_SIZE: usize = TARGET_SAMPLE_RATE as usize * FRAME_DURATION_MS as usize / 1000;
