//! Bit-level writer and reader for sub-byte packing

use alloc::vec::Vec;

pub struct BitWriter {
    data: Vec<u8>,
    current_byte: u8,
    bit_count: u8, // 0..8 bits currently filled in current_byte (MSB first)
}

impl BitWriter {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            current_byte: 0,
            bit_count: 0,
        }
    }

    pub fn with_capacity(bytes: usize) -> Self {
        Self {
            data: Vec::with_capacity(bytes),
            current_byte: 0,
            bit_count: 0,
        }
    }

    /// Write `count` bits from `value` (bits taken from lowest `count` bits of `value`, written MSB-first)
    pub fn write_bits(&mut self, value: u32, count: u8) {
        if count == 0 {
            return;
        }
        for i in (0..count).rev() {
            let bit = ((value >> i) & 1) as u8;
            self.current_byte = (self.current_byte << 1) | bit;
            self.bit_count += 1;
            if self.bit_count == 8 {
                self.data.push(self.current_byte);
                self.current_byte = 0;
                self.bit_count = 0;
            }
        }
    }

    /// Write 1 bit
    #[inline(always)]
    pub fn write_bit(&mut self, bit: bool) {
        self.current_byte = (self.current_byte << 1) | (bit as u8);
        self.bit_count += 1;
        if self.bit_count == 8 {
            self.data.push(self.current_byte);
            self.current_byte = 0;
            self.bit_count = 0;
        }
    }

    /// Write full byte
    #[inline(always)]
    pub fn write_byte(&mut self, byte: u8) {
        self.write_bits(byte as u32, 8);
    }

    /// Write slice of bytes
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_byte(b);
        }
    }

    /// Finish writing and return padded bytes (zeros pad the final incomplete byte)
    pub fn finish(mut self) -> Vec<u8> {
        if self.bit_count > 0 {
            self.current_byte <<= 8 - self.bit_count;
            self.data.push(self.current_byte);
        }
        self.data
    }

    /// Current number of bits written
    pub fn total_bits(&self) -> usize {
        self.data.len() * 8 + self.bit_count as usize
    }
}

pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8, // 0..8 (bits read from current byte, MSB first)
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    /// Read single bit
    pub fn read_bit(&mut self) -> Option<bool> {
        if self.byte_pos >= self.data.len() {
            return None;
        }
        let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Some(bit != 0)
    }

    /// Read `count` bits (up to 32)
    pub fn read_bits(&mut self, count: u8) -> Option<u32> {
        if count == 0 {
            return Some(0);
        }
        let mut result = 0u32;
        for _ in 0..count {
            let bit = self.read_bit()?;
            result = (result << 1) | (bit as u32);
        }
        Some(result)
    }

    /// Read 1 byte
    pub fn read_byte(&mut self) -> Option<u8> {
        self.read_bits(8).map(|v| v as u8)
    }

    /// Check if more bits are available
    pub fn has_more(&self) -> bool {
        self.byte_pos < self.data.len()
    }

    /// Remaining bits
    pub fn remaining_bits(&self) -> usize {
        if self.byte_pos >= self.data.len() {
            0
        } else {
            (self.data.len() - self.byte_pos) * 8 - self.bit_pos as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_stream_roundtrip() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b101, 3);
        writer.write_bit(false);
        writer.write_bit(true);
        writer.write_bits(0xABCD, 16);
        writer.write_byte(0x42);

        let data = writer.finish();

        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(3), Some(0b101));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_bits(16), Some(0xABCD));
        assert_eq!(reader.read_byte(), Some(0x42));
    }
}
