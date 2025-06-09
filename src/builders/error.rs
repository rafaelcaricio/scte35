//! Error types for the builder API.

use std::error::Error;
use std::fmt;
use std::time::Duration;

/// Errors that can occur during message building.
#[derive(Debug, Clone, PartialEq)]
pub enum BuilderError {
    /// A required field is missing.
    MissingRequiredField(&'static str),
    /// An invalid value was provided for a field.
    InvalidValue {
        /// The name of the field that had an invalid value.
        field: &'static str,
        /// A description of why the value is invalid.
        reason: String
    },
    /// A duration value is too large to fit in the SCTE-35 format.
    DurationTooLarge {
        /// The name of the field that had a duration that was too large.
        field: &'static str,
        /// The duration that was too large.
        duration: Duration
    },
    /// A UPID has an invalid length for its type.
    InvalidUpidLength {
        /// The expected length for this UPID type.
        expected: usize,
        /// The actual length provided.
        actual: usize
    },
    /// Too many components were specified.
    InvalidComponentCount {
        /// The maximum number of components allowed.
        max: usize,
        /// The actual number of components provided.
        actual: usize
    },
}

impl fmt::Display for BuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuilderError::MissingRequiredField(field) => 
                write!(f, "Required field '{}' is missing", field),
            BuilderError::InvalidValue { field, reason } => 
                write!(f, "Invalid value for field '{}': {}", field, reason),
            BuilderError::DurationTooLarge { field, duration } => 
                write!(f, "Duration for field '{}' is too large: {:?} exceeds 33-bit PTS limit", field, duration),
            BuilderError::InvalidUpidLength { expected, actual } => 
                write!(f, "Invalid UPID length: expected {} bytes, got {}", expected, actual),
            BuilderError::InvalidComponentCount { max, actual } => 
                write!(f, "Too many components: maximum {}, got {}", max, actual),
        }
    }
}

impl Error for BuilderError {}

/// Result type for builder operations.
pub type BuilderResult<T> = Result<T, BuilderError>;

/// Helper trait to convert Duration to 90kHz PTS ticks.
pub(crate) trait DurationExt {
    /// Convert duration to PTS ticks (90kHz clock).
    fn to_pts_ticks(&self) -> u64;
}

impl DurationExt for Duration {
    fn to_pts_ticks(&self) -> u64 {
        self.as_secs() * 90_000 + (self.subsec_nanos() as u64 * 90_000 / 1_000_000_000)
    }
}