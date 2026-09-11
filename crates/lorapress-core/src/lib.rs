//! lorapress-core: Ultra-compact payload compressor for LoRa/Meshtastic

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod airtime;
pub mod bitstream;
pub mod detector;
pub mod envelope;
pub mod image;
pub mod miren;
pub mod structured;
pub mod text;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    BufferTooSmall,
    InvalidData,
    UnsupportedType,
    DecompressionFailed,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer is too small"),
            Self::InvalidData => write!(f, "Invalid payload data"),
            Self::UnsupportedType => write!(f, "Unsupported payload type"),
            Self::DecompressionFailed => write!(f, "Decompression failed"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
