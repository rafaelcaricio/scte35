//! Error types for encoding operations.

use std::error::Error;
use std::fmt;

/// Result type for encoding operations.
pub type EncodingResult<T> = Result<T, EncodingError>;

/// Errors that can occur during encoding operations.
#[derive(Debug, Clone, PartialEq)]
pub enum EncodingError {
    /// Buffer overflow during encoding.
    BufferOverflow {
        /// Number of bytes needed.
        needed: usize,
        /// Number of bytes available.
        available: usize,
    },

    /// Invalid field value that cannot be encoded.
    InvalidFieldValue {
        /// Name of the field with invalid value.
        field: &'static str,
        /// String representation of the invalid value.
        value: String,
    },

    /// Missing required field for encoding.
    MissingRequiredField {
        /// Name of the missing field.
        field: &'static str,
    },

    /// Value exceeds the maximum allowed for its bit width.
    ValueTooLarge {
        /// Name of the field.
        field: &'static str,
        /// Maximum allowed value.
        max_value: u64,
        /// Actual value provided.
        actual_value: u64,
    },

    /// IO error during encoding.
    IoError(String),
}

impl fmt::Display for EncodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodingError::BufferOverflow { needed, available } => {
                write!(
                    f,
                    "Buffer overflow: needed {} bytes, had {}",
                    needed, available
                )
            }
            EncodingError::InvalidFieldValue { field, value } => {
                write!(f, "Invalid field value: {} = {}", field, value)
            }
            EncodingError::MissingRequiredField { field } => {
                write!(f, "Missing required field: {}", field)
            }
            EncodingError::ValueTooLarge {
                field,
                max_value,
                actual_value,
            } => {
                write!(
                    f,
                    "Value too large for field {}: {} > {} (max)",
                    field, actual_value, max_value
                )
            }
            EncodingError::IoError(msg) => {
                write!(f, "IO error: {}", msg)
            }
        }
    }
}

impl Error for EncodingError {}

impl From<std::io::Error> for EncodingError {
    fn from(err: std::io::Error) -> Self {
        EncodingError::IoError(err.to_string())
    }
}
