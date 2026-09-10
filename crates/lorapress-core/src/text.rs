//! LPC-Text v3: Prefix-free variable length character & dictionary encoder with End-of-Stream token

use alloc::string::String;
use alloc::vec::Vec;
use crate::bitstream::{BitReader, BitWriter};
use crate::Error;

/// High priority phrases/words (64 entries -> 6 bits index)
pub static RADIO_DICTIONARY: &[&str] = &[
    ", ",
    ". ",
    "! ",
    "? ",
    "прием",
    "база",
    "как слышно",
    "слышу хорошо",
    "принял",
    "отбой",
    "точка",
    "координаты",
    "маяк",
    "тревога",
    "SOS",
    "Roger",
    "73",
    "QTH",
    "QRZ",
    "QSL",
    "да",
    "нет",
    "в пути",
    "на месте",
    "на связи",
    "аккумулятор",
    "заряд",
    "температура",
    "высота",
    "скорость",
    "сеть",
    "пакет",
    "узел",
    "нода",
    "канал",
    "тест",
    "проверка связи",
    "все чисто",
    "внимание",
    "понял",
    "жду",
    "выдвигаюсь",
    "сообщение",
    "meshtastic",
    "lora",
    "http://",
    "https://",
    "://",
    ".com",
    ".ru",
    ".org",
    "status",
    "battery",
    "temp",
    "latitude",
    "longitude",
    "speed",
    "altitude",
    "channel",
    "node",
    "ping",
    "pong",
    "ack",
    "bye",
];

// Cyrillic table (33 lowercase characters: а-я, ё)
const CYRILLIC_TABLE: &[char] = &[
    'о', 'е', 'а', 'и', 'н', 'т', 'с', 'р', 'в', 'л', 
    'к', 'м', 'д', 'п', 'у', 'я', 'ы', 'ь', 'г', 'з', 
    'б', 'ч', 'й', 'х', 'ж', 'ш', 'ю', 'ц', 'щ', 'э', 
    'ф', 'ъ', 'ё',
];

// Latin table (26 lowercase characters: a-z)
const LATIN_TABLE: &[char] = &[
    'e', 't', 'a', 'o', 'i', 'n', 's', 'h', 'r', 'd', 
    'l', 'c', 'u', 'm', 'w', 'f', 'g', 'y', 'p', 'b', 
    'v', 'k', 'j', 'x', 'q', 'z',
];

// Symbols / Digits / Punctuation (32 entries -> 5 bits index)
const SYMBOLS_TABLE: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    '.', ',', '!', '?', '-', ':', ';', '/', '(', ')',
    '@', '#', '$', '%', '&', '*', '+', '=', '"', '\'',
    '_', '\n',
];

// Token Types (Prefix tags):
// 00 -> Space (' ')
// 01 -> Dictionary Match: 01 + [6 bits dict index]
// 10 -> Cyrillic char: 10 + [is_uppercase: 1 bit] + [char_idx: 6 bits]
// 110 -> Latin char: 110 + [is_uppercase: 1 bit] + [char_idx: 5 bits]
// 1110 -> Symbol / Digit: 1110 + [symbol_idx: 5 bits]
// 11110 -> Raw UTF-8 Byte fallback: 11110 + [8 bits byte]
// 11111 -> End of Stream (EOS marker)

pub struct TextCompressor;

impl TextCompressor {
    pub fn compress(text: &str) -> Vec<u8> {
        let mut writer = BitWriter::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 1. Try dictionary phrase match first (longest match)
            if let Some((dict_idx, match_len)) = Self::find_dict_match(&chars[i..]) {
                writer.write_bits(0b01, 2);
                writer.write_bits(dict_idx as u32, 6);
                i += match_len;
                continue;
            }

            let c = chars[i];
            i += 1;

            // 2. Space
            if c == ' ' {
                writer.write_bits(0b00, 2);
                continue;
            }

            // 3. Cyrillic
            let c_lower = c.to_lowercase().next().unwrap_or(c);
            if let Some(idx) = CYRILLIC_TABLE.iter().position(|&x| x == c_lower) {
                let is_upper = c.is_uppercase();
                writer.write_bits(0b10, 2);
                writer.write_bit(is_upper);
                writer.write_bits(idx as u32, 6);
                continue;
            }

            // 4. Latin
            if let Some(idx) = LATIN_TABLE.iter().position(|&x| x == c_lower) {
                let is_upper = c.is_uppercase();
                writer.write_bits(0b110, 3);
                writer.write_bit(is_upper);
                writer.write_bits(idx as u32, 5);
                continue;
            }

            // 5. Symbols & Digits
            if let Some(idx) = SYMBOLS_TABLE.iter().position(|&x| x == c) {
                writer.write_bits(0b1110, 4);
                writer.write_bits(idx as u32, 5);
                continue;
            }

            // 6. Raw UTF-8 byte fallback
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            for b in s.as_bytes() {
                writer.write_bits(0b11110, 5);
                writer.write_byte(*b);
            }
        }

        // Write explicit End of Stream marker (5 bits of 1s: 11111)
        writer.write_bits(0b11111, 5);

        writer.finish()
    }

    pub fn decompress(data: &[u8]) -> Result<String, Error> {
        let mut reader = BitReader::new(data);
        let mut result = String::new();
        let mut raw_utf8_buf = Vec::new();

        while reader.remaining_bits() >= 2 {
            let b0 = reader.read_bit().ok_or(Error::InvalidData)?;
            let b1 = reader.read_bit().ok_or(Error::InvalidData)?;

            match (b0, b1) {
                (false, false) => {
                    // 00 -> Space
                    Self::flush_utf8(&mut raw_utf8_buf, &mut result)?;
                    result.push(' ');
                }
                (false, true) => {
                    // 01 -> Dict
                    Self::flush_utf8(&mut raw_utf8_buf, &mut result)?;
                    let dict_idx = reader.read_bits(6).ok_or(Error::InvalidData)? as usize;
                    if dict_idx >= RADIO_DICTIONARY.len() {
                        return Err(Error::InvalidData);
                    }
                    result.push_str(RADIO_DICTIONARY[dict_idx]);
                }
                (true, false) => {
                    // 10 -> Cyrillic
                    Self::flush_utf8(&mut raw_utf8_buf, &mut result)?;
                    let is_upper = reader.read_bit().ok_or(Error::InvalidData)?;
                    let idx = reader.read_bits(6).ok_or(Error::InvalidData)? as usize;
                    let mut ch = *CYRILLIC_TABLE.get(idx).ok_or(Error::InvalidData)?;
                    if is_upper {
                        ch = ch.to_uppercase().next().unwrap_or(ch);
                    }
                    result.push(ch);
                }
                (true, true) => {
                    // 11 -> Latin (110), Symbol (1110), Raw (11110), EOS (11111)
                    let b2 = reader.read_bit().ok_or(Error::InvalidData)?;
                    if !b2 {
                        // 110 -> Latin
                        Self::flush_utf8(&mut raw_utf8_buf, &mut result)?;
                        let is_upper = reader.read_bit().ok_or(Error::InvalidData)?;
                        let idx = reader.read_bits(5).ok_or(Error::InvalidData)? as usize;
                        let mut ch = *LATIN_TABLE.get(idx).ok_or(Error::InvalidData)?;
                        if is_upper {
                            ch = ch.to_uppercase().next().unwrap_or(ch);
                        }
                        result.push(ch);
                    } else {
                        // 111 -> Symbol (1110), Raw (11110), EOS (11111)
                        let b3 = reader.read_bit().ok_or(Error::InvalidData)?;
                        if !b3 {
                            // 1110 -> Symbol
                            Self::flush_utf8(&mut raw_utf8_buf, &mut result)?;
                            let idx = reader.read_bits(5).ok_or(Error::InvalidData)? as usize;
                            let ch = *SYMBOLS_TABLE.get(idx).ok_or(Error::InvalidData)?;
                            result.push(ch);
                        } else {
                            // 1111 -> Raw (0) or EOS (1)
                            let b4 = reader.read_bit().ok_or(Error::InvalidData)?;
                            if !b4 {
                                // 11110 -> Raw byte
                                let byte = reader.read_byte().ok_or(Error::InvalidData)?;
                                raw_utf8_buf.push(byte);
                            } else {
                                // 11111 -> End of Stream
                                break;
                            }
                        }
                    }
                }
            }
        }

        Self::flush_utf8(&mut raw_utf8_buf, &mut result)?;
        Ok(result)
    }

    fn flush_utf8(buf: &mut Vec<u8>, result: &mut String) -> Result<(), Error> {
        if !buf.is_empty() {
            if let Ok(s) = core::str::from_utf8(buf) {
                result.push_str(s);
                buf.clear();
            } else {
                return Err(Error::InvalidData);
            }
        }
        Ok(())
    }

    fn find_dict_match(chars: &[char]) -> Option<(usize, usize)> {
        let mut best_match: Option<(usize, usize)> = None;

        for (idx, &phrase) in RADIO_DICTIONARY.iter().enumerate() {
            let phrase_chars: Vec<char> = phrase.chars().collect();
            if phrase_chars.len() >= 2 && chars.starts_with(&phrase_chars) {
                if let Some((_, best_len)) = best_match {
                    if phrase_chars.len() > best_len {
                        best_match = Some((idx, phrase_chars.len()));
                    }
                } else {
                    best_match = Some((idx, phrase_chars.len()));
                }
            }
        }

        best_match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_roundtrip() {
        let cases = [
            "База, как слышно? Еду на точку 5. Прием.",
            "Внимание всем! SOS, координаты потеряны, нужен маяк.",
            "Привет, да, скоро буду на месте!",
            "Roger that base, moving to checkpoint Alpha. 73!",
            "Hey, are you on channel 2? Battery is at 80%.",
            "Тест 12345 @ # $ % & * + = ! ? : ; . , /",
        ];

        for text in cases {
            let compressed = TextCompressor::compress(text);
            let decompressed = TextCompressor::decompress(&compressed).unwrap();
            assert_eq!(text, decompressed.as_str(), "Failed for case: {}", text);
        }
    }
}
