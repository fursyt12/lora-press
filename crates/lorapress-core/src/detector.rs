//! Universal Multi-Algorithm Compressor & Decompressor (FFmpeg-style engine)

use alloc::vec::Vec;
use crate::envelope::{EnvelopeHeader, PayloadType};
use crate::image::ClusterImageCodec;
use crate::structured::CompactJsonCodec;
use crate::text::TextCompressor;
use crate::Error;
use miniz_oxide::deflate::compress_to_vec;
use miniz_oxide::inflate::decompress_to_vec;

#[derive(Debug, Clone)]
pub struct CompressionReport {
    pub payload_type: PayloadType,
    pub original_size: usize,
    pub compressed_size: usize,
    pub ratio_percent: f64,
}

pub struct UniversalEngine;

impl UniversalEngine {
    /// Compresses any input data into optimal LoRaPress binary format
    pub fn compress(input: &[u8]) -> (EnvelopeHeader, Vec<u8>) {
        if input.is_empty() {
            return (EnvelopeHeader::new(PayloadType::Raw, 0), Vec::new());
        }

        // Strategy Race: Run all relevant candidates and pick the smallest one
        let mut candidates: Vec<(EnvelopeHeader, Vec<u8>)> = Vec::new();

        // 1. Candidate: Raw fallback
        candidates.push((EnvelopeHeader::new(PayloadType::Raw, 0), input.to_vec()));

        // 2. Candidate: Check if valid UTF-8
        if let Ok(s) = core::str::from_utf8(input) {
            let s_trimmed = s.trim();

            // 2a. If JSON object
            if s_trimmed.starts_with('{') && s_trimmed.ends_with('}') {
                if let Ok(json_comp) = CompactJsonCodec::compress_json_str(s_trimmed) {
                    candidates.push((EnvelopeHeader::new(PayloadType::JsonCompact, 0), json_comp));
                }
            }

            // 2b. Micro Text LPC (super fast and efficient for short strings)
            let text_lpc = TextCompressor::compress(s);
            candidates.push((EnvelopeHeader::new(PayloadType::TextMicro, 0), text_lpc));

            // 2c. Deflate/Zlib (for long texts & arbitrary JSONs > 100-200 bytes)
            if input.len() > 60 {
                let deflated = compress_to_vec(input, 10);
                let p_type = if s_trimmed.starts_with('{') {
                    PayloadType::JsonDeflate
                } else {
                    PayloadType::TextDeflate
                };
                candidates.push((EnvelopeHeader::new(p_type, 0), deflated));
            }
        } else {
            // Binary data > 60 bytes: try Deflate
            if input.len() > 60 {
                let deflated = compress_to_vec(input, 10);
                candidates.push((EnvelopeHeader::new(PayloadType::Raw, 1), deflated));
            }
        }

        // Pick the candidate with the minimum total byte size (including 1 byte header)
        candidates.into_iter().min_by_key(|(_, data)| data.len()).unwrap()
    }

    /// Decompresses any LoRaPress packet back to its original form
    pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let header = EnvelopeHeader::deserialize(data[0])?;
        let payload = &data[1..];

        match header.payload_type {
            PayloadType::Raw => {
                if header.sub_variant == 1 {
                    // Deflated binary
                    decompress_to_vec(payload).map_err(|_| Error::DecompressionFailed)
                } else {
                    Ok(payload.to_vec())
                }
            }
            PayloadType::TextMicro => {
                let s = TextCompressor::decompress(payload)?;
                Ok(s.into_bytes())
            }
            PayloadType::TextDeflate | PayloadType::JsonDeflate => {
                decompress_to_vec(payload).map_err(|_| Error::DecompressionFailed)
            }
            PayloadType::JsonCompact => {
                let val = CompactJsonCodec::decompress_to_json(payload)?;
                let s = serde_json::to_string(&val).map_err(|_| Error::InvalidData)?;
                Ok(s.into_bytes())
            }
            PayloadType::ImageCluster => {
                // Return raw image bytes or bitmap representation
                let (_w, _h, _pixels) = ClusterImageCodec::decode_4x4_clustered(payload)?;
                Ok(payload.to_vec())
            }
            _ => Err(Error::UnsupportedType),
        }
    }
}
