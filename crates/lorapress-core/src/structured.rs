//! Micro JSON / Structured KV compression into bitpacked payloads
//! Compresses JSON objects with known/common IoT & Meshtastic telemetry keys down to 10-25 bytes!

use alloc::string::String;
use alloc::vec::Vec;
use crate::bitstream::{BitReader, BitWriter};
use crate::Error;
use serde_json::Value;

pub static JSON_TELEMETRY_KEYS: &[&str] = &[
    "lat", "lon", "alt", "temp", "hum", "press", 
    "batt", "voltage", "speed", "heading", "rssi", 
    "snr", "hops", "time", "node", "msg", "type"
];

pub struct CompactJsonCodec;

impl CompactJsonCodec {
    /// Compresses typical IoT JSON objects into ultra-compact binary format
    pub fn compress_json_str(json_str: &str) -> Result<Vec<u8>, Error> {
        let val: Value = serde_json::from_str(json_str).map_err(|_| Error::InvalidData)?;
        Self::compress_value(&val)
    }

    pub fn compress_value(val: &Value) -> Result<Vec<u8>, Error> {
        let mut writer = BitWriter::new();
        match val {
            Value::Object(map) => {
                writer.write_bits(0b00, 2); // Object type
                writer.write_bits(map.len() as u32, 4); // Up to 16 keys

                for (key, v) in map {
                    // 1. Key encoding: check if in key dictionary
                    if let Some(idx) = JSON_TELEMETRY_KEYS.iter().position(|&k| k == key.as_str()) {
                        writer.write_bit(true); // Dict key
                        writer.write_bits(idx as u32, 5); // 5 bits key id
                    } else {
                        writer.write_bit(false); // Raw key
                        let bytes = key.as_bytes();
                        writer.write_bits(bytes.len() as u32, 4);
                        writer.write_bytes(bytes);
                    }

                    // 2. Value encoding
                    Self::encode_val(&mut writer, v)?;
                }
            }
            _ => return Err(Error::UnsupportedType),
        }

        Ok(writer.finish())
    }

    fn encode_val(writer: &mut BitWriter, val: &Value) -> Result<(), Error> {
        match val {
            Value::Null => {
                writer.write_bits(0b000, 3);
            }
            Value::Bool(b) => {
                writer.write_bits(0b001, 3);
                writer.write_bit(*b);
            }
            Value::Number(num) => {
                if let Some(i) = num.as_i64() {
                    if i >= 0 && i < 256 {
                        writer.write_bits(0b010, 3); // u8
                        writer.write_byte(i as u8);
                    } else if i >= -32768 && i <= 32767 {
                        writer.write_bits(0b011, 3); // i16
                        writer.write_bytes(&(i as i16).to_le_bytes());
                    } else {
                        writer.write_bits(0b100, 3); // i32
                        writer.write_bytes(&(i as i32).to_le_bytes());
                    }
                } else if let Some(f) = num.as_f64() {
                    // Check if it fits into fixed point (e.g. coordinates or 2-decimal floats like temp 21.4)
                    let fixed_e4 = (f * 10000.0).round() as i64;
                    if (fixed_e4 as f64 / 10000.0 - f).abs() < 0.0001 && fixed_e4 >= -2147483648 && fixed_e4 <= 2147483647 {
                        writer.write_bits(0b101, 3); // Fixed-point 1e4 (i32)
                        writer.write_bytes(&(fixed_e4 as i32).to_le_bytes());
                    } else {
                        writer.write_bits(0b110, 3); // Raw f32
                        writer.write_bytes(&(f as f32).to_le_bytes());
                    }
                }
            }
            Value::String(s) => {
                writer.write_bits(0b111, 3); // String
                let bytes = s.as_bytes();
                writer.write_bits(bytes.len() as u32, 6); // Up to 63 bytes string
                writer.write_bytes(bytes);
            }
            _ => return Err(Error::UnsupportedType),
        }
        Ok(())
    }

    pub fn decompress_to_json(data: &[u8]) -> Result<Value, Error> {
        let mut reader = BitReader::new(data);
        let val_type = reader.read_bits(2).ok_or(Error::InvalidData)?;
        if val_type != 0b00 {
            return Err(Error::UnsupportedType);
        }

        let num_entries = reader.read_bits(4).ok_or(Error::InvalidData)? as usize;
        let mut map = serde_json::Map::new();

        for _ in 0..num_entries {
            let is_dict_key = reader.read_bit().ok_or(Error::InvalidData)?;
            let key = if is_dict_key {
                let k_idx = reader.read_bits(5).ok_or(Error::InvalidData)? as usize;
                JSON_TELEMETRY_KEYS.get(k_idx).copied().ok_or(Error::InvalidData)?.to_string()
            } else {
                let k_len = reader.read_bits(4).ok_or(Error::InvalidData)? as usize;
                let mut bytes = Vec::with_capacity(k_len);
                for _ in 0..k_len {
                    bytes.push(reader.read_byte().ok_or(Error::InvalidData)?);
                }
                String::from_utf8(bytes).map_err(|_| Error::InvalidData)?
            };

            let val = Self::decode_val(&mut reader)?;
            map.insert(key, val);
        }

        Ok(Value::Object(map))
    }

    fn decode_val(reader: &mut BitReader) -> Result<Value, Error> {
        let tag = reader.read_bits(3).ok_or(Error::InvalidData)?;
        match tag {
            0b000 => Ok(Value::Null),
            0b001 => {
                let b = reader.read_bit().ok_or(Error::InvalidData)?;
                Ok(Value::Bool(b))
            }
            0b010 => {
                let byte = reader.read_byte().ok_or(Error::InvalidData)?;
                Ok(serde_json::json!(byte))
            }
            0b011 => {
                let b0 = reader.read_byte().ok_or(Error::InvalidData)?;
                let b1 = reader.read_byte().ok_or(Error::InvalidData)?;
                let val = i16::from_le_bytes([b0, b1]);
                Ok(serde_json::json!(val))
            }
            0b100 => {
                let mut buf = [0u8; 4];
                for b in &mut buf { *b = reader.read_byte().ok_or(Error::InvalidData)?; }
                let val = i32::from_le_bytes(buf);
                Ok(serde_json::json!(val))
            }
            0b101 => {
                let mut buf = [0u8; 4];
                for b in &mut buf { *b = reader.read_byte().ok_or(Error::InvalidData)?; }
                let fixed = i32::from_le_bytes(buf);
                let f = fixed as f64 / 10000.0;
                Ok(serde_json::json!(f))
            }
            0b110 => {
                let mut buf = [0u8; 4];
                for b in &mut buf { *b = reader.read_byte().ok_or(Error::InvalidData)?; }
                let f = f32::from_le_bytes(buf);
                Ok(serde_json::json!(f))
            }
            0b111 => {
                let s_len = reader.read_bits(6).ok_or(Error::InvalidData)? as usize;
                let mut bytes = Vec::with_capacity(s_len);
                for _ in 0..s_len {
                    bytes.push(reader.read_byte().ok_or(Error::InvalidData)?);
                }
                let s = String::from_utf8(bytes).map_err(|_| Error::InvalidData)?;
                Ok(Value::String(s))
            }
            _ => Err(Error::InvalidData),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_telemetry_roundtrip() {
        let json_str = r#"{"lat":55.7512,"lon":37.6184,"alt":154,"batt":89,"temp":21.4}"#;
        let compressed = CompactJsonCodec::compress_json_str(json_str).unwrap();
        
        // Original is 65 bytes, compressed is ~21 bytes! (67% compression)
        assert!(compressed.len() <= 22, "Compressed size: {}", compressed.len());

        let decompressed = CompactJsonCodec::decompress_to_json(&compressed).unwrap();
        assert_eq!(decompressed["alt"], 154);
        assert_eq!(decompressed["batt"], 89);
    }
}
