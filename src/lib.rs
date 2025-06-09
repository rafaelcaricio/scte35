#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#![warn(missing_docs)]

use std::io;

// Internal modules
mod bit_reader;
mod commands;

// Public modules
/// Builder pattern API for creating SCTE-35 messages from scratch.
pub mod builders;
pub mod descriptors;
pub mod parser;
pub mod time;
pub mod types;
pub mod upid;

// CRC validation module - only included when feature is enabled
#[cfg(feature = "crc-validation")]
pub mod crc;

// Serde support module - only included when feature is enabled
#[cfg(feature = "serde")]
pub mod serde;

// Re-export commonly used CRC functions for convenience - only when available
#[cfg(feature = "crc-validation")]
pub use crc::{validate_message_crc, CrcValidatable};

// Re-export main types and functions for ease of use
pub use parser::parse_splice_info_section;

// Re-export main types
pub use types::{
    BandwidthReservation, ComponentSplice, PrivateCommand, SegmentationType, SpliceCommand,
    SpliceInfoSection, SpliceInsert, SpliceInsertComponent, SpliceNull, SpliceSchedule, TimeSignal,
};

// Re-export time types
pub use time::{BreakDuration, DateTime, SpliceTime};

// Re-export UPID types
pub use upid::SegmentationUpidType;

// Re-export descriptor types
pub use descriptors::{SegmentationDescriptor, SpliceDescriptor};

/// Validates the CRC-32 checksum of an SCTE-35 message.
///
/// This is a convenience function that wraps [`crc::validate_message_crc`].
/// For more CRC functionality, use the [`crc`] module directly.
///
/// # Arguments
///
/// * `buffer` - The complete SCTE-35 message bytes
///
/// # Returns
///
/// * `Ok(true)` - CRC validation passed
/// * `Ok(false)` - CRC validation not available (feature disabled)
/// * `Err(io::Error)` - Parse error or validation error
///
/// # Example
///
/// ```rust
/// use scte35_parsing::validate_scte35_crc;
/// use base64::{Engine, engine::general_purpose};
///
/// let base64_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
/// let buffer = general_purpose::STANDARD.decode(base64_message).unwrap();
///
/// match validate_scte35_crc(&buffer) {
///     Ok(true) => println!("CRC validation passed"),
///     Ok(false) => println!("CRC validation not available"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
#[cfg(feature = "crc-validation")]
pub fn validate_scte35_crc(buffer: &[u8]) -> Result<bool, io::Error> {
    crate::crc::validate_message_crc(buffer)
}

/// Fallback function when CRC validation is not available.
#[cfg(not(feature = "crc-validation"))]
pub fn validate_scte35_crc(_buffer: &[u8]) -> Result<bool, io::Error> {
    Ok(false)
}

// Add CRC validation methods to SpliceInfoSection when CRC feature is available
#[cfg(feature = "crc-validation")]
impl crc::CrcValidatable for types::SpliceInfoSection {
    fn validate_crc(&self, buffer: &[u8]) -> Result<bool, io::Error> {
        crate::crc::validate_message_crc(buffer)
    }

    fn get_crc(&self) -> u32 {
        self.crc_32
    }
}

// Import the test module from the old lib.rs
#[cfg(test)]
mod tests;
