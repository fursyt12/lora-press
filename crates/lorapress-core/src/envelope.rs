//! Envelope Header for LoRaPress packets (1 to 2 bytes overhead)

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PayloadType {
    Raw = 0x0,
    TextMicro = 0x1,       // Micro LPC token codec (< 80 bytes)
    TextDeflate = 0x2,     // Medium/Long text Deflate/LZ
    JsonCompact = 0x3,     // IoT Telemetry Schema/KV
    JsonDeflate = 0x4,     // Arbitrary large JSON Deflate
    ImageCluster = 0x5,    // Cluster / QuadTree Block Image
    VoiceCodec2 = 0x6,     // Codec2 voice frames
    MirenStream = 0x7,     // Miren ARKit 52 Blendshapes + Audio Stream
}

impl PayloadType {
    pub fn from_u8(val: u8) -> Result<Self, Error> {
        match val & 0x07 {
            0x0 => Ok(Self::Raw),
            0x1 => Ok(Self::TextMicro),
            0x2 => Ok(Self::TextDeflate),
            0x3 => Ok(Self::JsonCompact),
            0x4 => Ok(Self::JsonDeflate),
            0x5 => Ok(Self::ImageCluster),
            0x6 => Ok(Self::VoiceCodec2),
            0x7 => Ok(Self::MirenStream),
            _ => Err(Error::UnsupportedType),
        }
    }
}

/// Header Layout (1 Byte standard):
/// [Bit 7..5 (3 bits)]: Payload Type (8 types)
/// [Bit 4..3 (2 bits)]: Sub-variant / Flags (e.g. Dict ID, resolution variant)
/// [Bit 2 (1 bit)]: Is Extended Header (if 1, followed by byte 2 for fragment/crc)
/// [Bit 1..0 (2 bits)]: Custom field (e.g. compression level or raw flag)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeHeader {
    pub payload_type: PayloadType,
    pub sub_variant: u8, // 0..3
    pub is_extended: bool,
    pub aux: u8, // 0..3
}

impl EnvelopeHeader {
    pub fn new(payload_type: PayloadType, sub_variant: u8) -> Self {
        Self {
            payload_type,
            sub_variant: sub_variant & 0x03,
            is_extended: false,
            aux: 0,
        }
    }

    pub fn serialize(&self) -> u8 {
        let type_bits = (self.payload_type as u8 & 0x07) << 5;
        let sub_bits = (self.sub_variant & 0x03) << 3;
        let ext_bit = if self.is_extended { 1 << 2 } else { 0 };
        let aux_bits = self.aux & 0x03;

        type_bits | sub_bits | ext_bit | aux_bits
    }

    pub fn deserialize(byte: u8) -> Result<Self, Error> {
        let payload_type = PayloadType::from_u8((byte >> 5) & 0x07)?;
        let sub_variant = (byte >> 3) & 0x03;
        let is_extended = (byte & 0x04) != 0;
        let aux = byte & 0x03;

        Ok(Self {
            payload_type,
            sub_variant,
            is_extended,
            aux,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        for pt in [
            PayloadType::Raw,
            PayloadType::TextMicro,
            PayloadType::TextDeflate,
            PayloadType::JsonCompact,
            PayloadType::JsonDeflate,
            PayloadType::ImageCluster,
            PayloadType::VoiceCodec2,
            PayloadType::MirenStream,
        ] {
            for sub in 0..4 {
                let hdr = EnvelopeHeader::new(pt, sub);
                let encoded = hdr.serialize();
                let decoded = EnvelopeHeader::deserialize(encoded).unwrap();
                assert_eq!(hdr.payload_type, decoded.payload_type);
                assert_eq!(hdr.sub_variant, decoded.sub_variant);
            }
        }
    }
}
