//! Binary encoding support for SCTE-35 messages.
//!
//! This module provides functionality to encode SCTE-35 structures into their binary
//! wire format, complementing the parsing functionality with serialization capabilities.

/// Error types for encoding operations.
pub mod error;

/// Bit-level writer for encoding binary data.
pub mod writer;

/// Trait definitions for encodable types.
pub mod traits;

// Implementation modules
mod splice_info_section;
mod commands;
mod descriptors;
mod time;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod round_trip_tests;

// Re-export commonly used types
pub use error::{EncodingError, EncodingResult};
pub use traits::Encodable;
pub use writer::BitWriter;

// Re-export feature-gated traits
#[cfg(feature = "crc-validation")]
pub use traits::CrcEncodable;

#[cfg(feature = "base64")]
pub use traits::Base64Encodable;