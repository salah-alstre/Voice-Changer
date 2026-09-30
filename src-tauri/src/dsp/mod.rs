//! Real-time voice DSP. Pure Rust, no OS dependencies, fully unit-tested.

pub mod biquad;
pub mod chain;
pub mod dynamics;
pub mod effects;
pub mod noise;
pub mod params;
pub mod pitch;
pub mod presets;

pub use chain::VoiceChain;
pub use params::VoiceParams;
