//! Builder pattern API for creating SCTE-35 messages from scratch.
//!
//! This module provides type-safe builders for constructing valid SCTE-35 messages
//! with proper validation and ergonomic APIs. It follows the Builder pattern to
//! ensure messages are constructed correctly according to the SCTE-35 specification.

/// Error types for the builder API.
pub mod error;
/// Extensions for existing types to support the builder pattern.
pub mod extensions;
/// Builder for creating SCTE-35 splice information sections.
pub mod splice_info_section;
/// Builders for SCTE-35 splice commands.
pub mod commands;
/// Builders for SCTE-35 descriptors.
pub mod descriptors;
/// Time-related builder utilities for SCTE-35 messages.
pub mod time;

#[cfg(test)]
mod tests;

// Re-export builders at module level
pub use error::{BuilderError, BuilderResult};
pub use splice_info_section::SpliceInfoSectionBuilder;
pub use commands::*;
pub use descriptors::*;
pub use time::*;