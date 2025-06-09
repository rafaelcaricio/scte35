//! CRC validation module for SCTE-35 messages.
//!
//! This module provides CRC-32 validation functionality for SCTE-35 messages
//! using the MPEG-2 CRC algorithm as specified in the SCTE-35 standard.

use std::io::{self, ErrorKind};

#[cfg(feature = "crc-validation")]
use crc::{Crc, CRC_32_MPEG_2};

/// MPEG-2 CRC-32 algorithm instance for SCTE-35 validation
#[cfg(feature = "crc-validation")]
pub const MPEG_2: Crc<u32> = Crc::<u32>::new(&CRC_32_MPEG_2);

/// Validates CRC-32 checksum using MPEG-2 algorithm.
///
/// # Arguments
///
/// * `data` - The data bytes to validate (excluding CRC field)
/// * `expected_crc` - The expected CRC-32 value
///
/// # Returns
///
/// * `true` - CRC validation passed
/// * `false` - CRC validation failed
#[cfg(feature = "crc-validation")]
pub fn validate_crc(data: &[u8], expected_crc: u32) -> bool {
    let calculated_crc = MPEG_2.checksum(data);
    calculated_crc == expected_crc
}

/// Stub function when CRC validation is disabled.
#[cfg(not(feature = "crc-validation"))]
pub fn validate_crc(_data: &[u8], _expected_crc: u32) -> bool {
    false // Always return false when CRC validation is disabled
}

/// Calculates CRC-32 checksum for the given data.
///
/// # Arguments
///
/// * `data` - The data bytes to calculate CRC for
///
/// # Returns
///
/// * `Some(u32)` - Calculated CRC-32 value (when crc-validation feature is enabled)
/// * `None` - CRC calculation not available (when crc-validation feature is disabled)
#[cfg(feature = "crc-validation")]
pub fn calculate_crc(data: &[u8]) -> Option<u32> {
    Some(MPEG_2.checksum(data))
}

/// Stub function when CRC calculation is disabled.
#[cfg(not(feature = "crc-validation"))]
pub fn calculate_crc(_data: &[u8]) -> Option<u32> {
    None
}

/// Validates the CRC-32 checksum of a complete SCTE-35 message.
///
/// This function extracts the CRC from the last 4 bytes of the buffer
/// and validates it against the calculated CRC of the preceding data.
///
/// # Arguments
///
/// * `buffer` - The complete SCTE-35 message bytes
///
/// # Returns
///
/// * `Ok(true)` - CRC validation passed
/// * `Ok(false)` - CRC validation failed or not available
/// * `Err(io::Error)` - Buffer too short or other validation error
///
/// # Example
///
/// ```rust
/// use scte35::crc::validate_message_crc;
/// use data_encoding::BASE64;
///
/// let base64_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
/// let buffer = BASE64.decode(base64_message.as_bytes()).unwrap();
///
/// match validate_message_crc(&buffer) {
///     Ok(true) => println!("CRC validation passed"),
///     Ok(false) => println!("CRC validation failed or not available"),
///     Err(e) => eprintln!("Validation error: {}", e),
/// }
/// ```
pub fn validate_message_crc(buffer: &[u8]) -> Result<bool, io::Error> {
    if buffer.len() < 4 {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "Buffer too short to contain CRC-32 field",
        ));
    }

    // Extract CRC from the last 4 bytes (big-endian)
    let crc_bytes = &buffer[buffer.len() - 4..];
    let stored_crc = u32::from_be_bytes([crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]]);

    // Calculate CRC over the data (excluding CRC field)
    let data = &buffer[0..buffer.len() - 4];
    Ok(validate_crc(data, stored_crc))
}

/// Trait for types that can validate their CRC against original message data.
pub trait CrcValidatable {
    /// Validates the CRC-32 checksum against the original message data.
    ///
    /// # Arguments
    ///
    /// * `original_buffer` - The original message bytes used to parse this structure
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - CRC validation passed
    /// * `Ok(false)` - CRC validation disabled or failed
    /// * `Err(io::Error)` - Validation error
    fn validate_crc(&self, original_buffer: &[u8]) -> Result<bool, io::Error>;

    /// Returns the stored CRC-32 value from the parsed structure.
    fn get_crc(&self) -> u32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "crc-validation")]
    fn test_crc_calculation() {
        let test_data = b"Hello, SCTE-35!";
        let crc = calculate_crc(test_data);
        assert!(crc.is_some());

        // Validate the calculated CRC
        let calculated = crc.unwrap();
        assert!(validate_crc(test_data, calculated));
    }

    #[test]
    #[cfg(not(feature = "crc-validation"))]
    fn test_crc_disabled() {
        let test_data = b"Hello, SCTE-35!";
        let crc = calculate_crc(test_data);
        assert!(crc.is_none());

        // Should always return false when disabled
        assert!(!validate_crc(test_data, 0));
    }

    #[test]
    fn test_message_crc_validation_short_buffer() {
        let short_buffer = vec![0x01, 0x02]; // Too short for CRC
        let result = validate_message_crc(&short_buffer);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "crc-validation")]
    fn test_message_crc_validation() {
        // Create a test buffer with known CRC
        let mut test_data = vec![0xFC, 0x30, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00];

        // Calculate CRC for the data
        if let Some(calculated_crc) = calculate_crc(&test_data) {
            // Append CRC to create complete message
            test_data.extend_from_slice(&calculated_crc.to_be_bytes());

            // Validate the complete message
            let result = validate_message_crc(&test_data);
            assert!(result.is_ok());
            assert!(result.unwrap());
        }
    }
}
