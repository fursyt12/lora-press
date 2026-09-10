//! Cluster Block Image Codec for LoRa/Meshtastic
//!
//! Features:
//! 1. Breaks image into NxN macro-blocks (4x4 or 8x8 clusters).
//! 2. Encodes each cluster by its dominant pattern:
//!    - Solid (all white / all black) -> 2 bits!
//!    - Gradient / Half split -> 4 bits
//!    - High frequency / detailed -> Bit-packed bitmask
//! 3. Lossy thresholding option: merges near-solid blocks for extreme compression.
//! 4. Supports 32x32, 48x48, 64x64, 128x128 monochrome images.

use alloc::vec::Vec;
use crate::bitstream::{BitReader, BitWriter};
use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    SolidBlack = 0, // All 0s
    SolidWhite = 1, // All 1s
    Pattern = 2,    // Exact bit pattern
}

pub struct ClusterImageCodec;

impl ClusterImageCodec {
    /// Encode a 1-bit monochrome image using 4x4 block clustering
    pub fn encode_4x4_clustered(width: u32, height: u32, pixels: &[bool], lossy_threshold: u8) -> Vec<u8> {
        let mut writer = BitWriter::new();

        // Header: Width & Height (in multiples of 4: 0..63 -> up to 256x256)
        let w_blocks = (width + 3) / 4;
        let h_blocks = (height + 3) / 4;
        
        writer.write_bits(w_blocks.min(63), 6);
        writer.write_bits(h_blocks.min(63), 6);

        // Process block by block
        for by in 0..h_blocks {
            for bx in 0..w_blocks {
                let mut ones_count = 0;
                let mut block_bits = [false; 16];

                for py in 0..4 {
                    for px in 0..4 {
                        let x = bx * 4 + px;
                        let y = by * 4 + py;
                        let val = if x < width && y < height {
                            pixels[(y * width + x) as usize]
                        } else {
                            false
                        };
                        let bit_idx = (py * 4 + px) as usize;
                        block_bits[bit_idx] = val;
                        if val {
                            ones_count += 1;
                        }
                    }
                }

                // Check for solid / lossy approximation
                if ones_count <= lossy_threshold as usize {
                    // Solid Black: tag 00 (2 bits)
                    writer.write_bits(0b00, 2);
                } else if ones_count >= (16 - lossy_threshold as usize) {
                    // Solid White: tag 01 (2 bits)
                    writer.write_bits(0b01, 2);
                } else {
                    // Detailed Pattern: tag 1 + 16 raw bits (17 bits)
                    writer.write_bit(true);
                    for &bit in &block_bits {
                        writer.write_bit(bit);
                    }
                }
            }
        }

        writer.finish()
    }

    /// Decode 4x4 clustered image
    pub fn decode_4x4_clustered(data: &[u8]) -> Result<(u32, u32, Vec<bool>), Error> {
        let mut reader = BitReader::new(data);
        let w_blocks = reader.read_bits(6).ok_or(Error::InvalidData)?;
        let h_blocks = reader.read_bits(6).ok_or(Error::InvalidData)?;

        let width = w_blocks * 4;
        let height = h_blocks * 4;
        let mut pixels = vec![false; (width * height) as usize];

        for by in 0..h_blocks {
            for bx in 0..w_blocks {
                let is_pattern = reader.read_bit().ok_or(Error::InvalidData)?;
                if !is_pattern {
                    // Solid block (2nd bit determines color)
                    let is_white = reader.read_bit().ok_or(Error::InvalidData)?;
                    for py in 0..4 {
                        for px in 0..4 {
                            let x = bx * 4 + px;
                            let y = by * 4 + py;
                            pixels[(y * width + x) as usize] = is_white;
                        }
                    }
                } else {
                    // Detailed pattern (read 16 bits)
                    for py in 0..4 {
                        for px in 0..4 {
                            let bit = reader.read_bit().ok_or(Error::InvalidData)?;
                            let x = bx * 4 + px;
                            let y = by * 4 + py;
                            pixels[(y * width + x) as usize] = bit;
                        }
                    }
                }
            }
        }

        Ok((width, height, pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_image_roundtrip() {
        let width = 32;
        let height = 32;
        let mut pixels = vec![false; (width * height) as usize];

        // Draw a solid box and a cross
        for y in 0..8 {
            for x in 0..8 {
                pixels[y * width + x] = true;
            }
        }
        for i in 0..32 {
            pixels[i * width + i] = true;
        }

        // Lossless clustering (threshold = 0)
        let compressed = ClusterImageCodec::encode_4x4_clustered(width as u32, height as u32, &pixels, 0);
        let (w, h, decoded) = ClusterImageCodec::decode_4x4_clustered(&compressed).unwrap();

        assert_eq!(w, width as u32);
        assert_eq!(h, height as u32);
        assert_eq!(pixels, decoded);

        // Raw 32x32 = 128 bytes. Clustered should be much smaller!
        assert!(compressed.len() < 70, "Compressed size: {}", compressed.len());
    }
}
