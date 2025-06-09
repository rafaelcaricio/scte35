//! Trait definitions for encodable types.

use super::error::EncodingResult;
use super::writer::BitWriter;

/// Trait for types that can be encoded to SCTE-35 binary format.
pub trait Encodable {
    /// Encode the structure to binary SCTE-35 format.
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()>;
    
    /// Calculate the encoded size in bytes.
    ///
    /// This should return the exact number of bytes that will be written
    /// when `encode` is called. This is used for pre-allocating buffers.
    fn encoded_size(&self) -> usize;
    
    /// Convenience method to encode to a new byte vector.
    fn encode_to_vec(&self) -> EncodingResult<Vec<u8>> {
        let mut writer = BitWriter::with_capacity(self.encoded_size());
        self.encode(&mut writer)?;
        Ok(writer.finish())
    }
}

/// Extension trait for encoding with CRC support.
#[cfg(feature = "crc-validation")]
pub trait CrcEncodable: Encodable {
    /// Encode with automatic CRC calculation and insertion.
    fn encode_with_crc(&self) -> EncodingResult<Vec<u8>>;
}

/// Extension trait for base64 encoding support.
#[cfg(feature = "base64")]
pub trait Base64Encodable: Encodable {
    /// Encode to base64 string.
    fn encode_base64(&self) -> EncodingResult<String> {
        use base64::{engine::general_purpose, Engine};
        let bytes = self.encode_to_vec()?;
        Ok(general_purpose::STANDARD.encode(bytes))
    }
    
    /// Encode with CRC and then to base64.
    #[cfg(feature = "crc-validation")]
    fn encode_base64_with_crc(&self) -> EncodingResult<String>
    where
        Self: CrcEncodable,
    {
        use base64::{engine::general_purpose, Engine};
        let bytes = self.encode_with_crc()?;
        Ok(general_purpose::STANDARD.encode(bytes))
    }
}