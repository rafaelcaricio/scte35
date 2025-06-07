# CRC Validation Enhancement

This document outlines how to introduce optional CRC validation using the `crc` crate for SCTE-35 payload validation.

## Overview

Currently, the SCTE-35 parsing library reads and stores the CRC-32 value from SCTE-35 messages but does not validate it against the actual message content. Adding CRC validation would improve data integrity verification and help detect corrupted or tampered messages.

## Implementation Plan

### 1. Add `crc` Dependency

Add the `crc` crate as an optional dependency that's included in default features:

```toml
[dependencies]
crc = { version = "3.0", optional = true }
base64 = { version = "0.21", optional = true }

[features]
default = ["crc-validation"]
crc-validation = ["crc"]
cli = ["base64", "crc-validation"]  # CLI automatically enables CRC validation
```

**Important**: The `cli` feature automatically enables `crc-validation` because:
- CLI users expect complete diagnostic information
- CRC validation is essential for debugging SCTE-35 messages
- The CLI tool should show whether messages are corrupted or valid

### 2. CRC Module Implementation

Create a dedicated `src/crc.rs` module to isolate all CRC-related functionality:

```rust
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
/// use scte35_parsing::crc::validate_message_crc;
/// use base64::{Engine, engine::general_purpose};
///
/// let base64_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
/// let buffer = general_purpose::STANDARD.decode(base64_message).unwrap();
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
            "Buffer too short to contain CRC-32 field"
        ));
    }

    // Extract CRC from the last 4 bytes (big-endian)
    let crc_bytes = &buffer[buffer.len() - 4..];
    let stored_crc = u32::from_be_bytes([
        crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]
    ]);

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

### 3. lib.rs Integration

Conditionally include the CRC module in `src/lib.rs` only when the feature is enabled:

```rust
//! # SCTE-35 Parsing Library
//! 
//! // ... existing module documentation ...

#![warn(missing_docs)]

use std::io::{self, ErrorKind};
use std::time::Duration;

// CRC validation module - only included when feature is enabled
#[cfg(feature = "crc-validation")]
pub mod crc;

// Re-export commonly used CRC functions for convenience - only when available
#[cfg(feature = "crc-validation")]
pub use crc::{validate_message_crc, CrcValidatable};

// ... rest of existing code ...
```

This approach means:
- The entire `crc` module is excluded from compilation when feature is disabled
- No need for feature flags inside the `crc.rs` file (except for specific stubs if needed)
- Much cleaner and simpler code

### 4. Integration Points

#### 4.1 Parser Function Enhancement

Modify `parse_splice_info_section()` to optionally validate CRC:

```rust
pub fn parse_splice_info_section(buffer: &[u8]) -> Result<SpliceInfoSection, io::Error> {
    // ... existing parsing logic ...

    let crc_32 = reader.read_rpchof(32)? as u32;

    // Validate CRC if feature is enabled - much cleaner!
    #[cfg(feature = "crc-validation")]
    {
        if !crc::validate_crc(&buffer[0..buffer.len() - 4], crc_32) {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("CRC validation failed. Expected: 0x{:08X}", crc_32)
            ));
        }
    }

    Ok(SpliceInfoSection {
        // ... existing fields ...
        crc_32,
    })
}
```

#### 4.2 Convenience Functions

Add top-level convenience functions with simple feature gating:

```rust
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
///     Ok(false) => println!("CRC validation failed or not available"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
#[cfg(feature = "crc-validation")]
pub fn validate_scte35_crc(buffer: &[u8]) -> Result<bool, io::Error> {
    crc::validate_message_crc(buffer)
}

/// Stub function when CRC validation is not available.
#[cfg(not(feature = "crc-validation"))]
pub fn validate_scte35_crc(_buffer: &[u8]) -> Result<bool, io::Error> {
    Ok(false) // CRC validation not available
}

### 5. Enhanced SpliceInfoSection

Add CRC validation methods to the main structure with minimal feature gating:

```rust
impl SpliceInfoSection {
    /// Validates the CRC-32 checksum against the original message data.
    ///
    /// # Arguments
    ///
    /// * `original_buffer` - The original message bytes used to parse this section
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - CRC validation passed
    /// * `Ok(false)` - CRC validation disabled or failed
    /// * `Err(io::Error)` - Validation error
    #[cfg(feature = "crc-validation")]
    pub fn validate_crc(&self, original_buffer: &[u8]) -> Result<bool, io::Error> {
        crc::validate_message_crc(original_buffer)
    }
    
    /// Stub function when CRC validation is not available.
    #[cfg(not(feature = "crc-validation"))]
    pub fn validate_crc(&self, _original_buffer: &[u8]) -> Result<bool, io::Error> {
        Ok(false) // CRC validation not available
    }
    
    /// Returns the stored CRC-32 value from the parsed section.
    pub fn get_crc(&self) -> u32 {
        self.crc_32
    }
}

// Only implement the trait when the feature is available
#[cfg(feature = "crc-validation")]
impl CrcValidatable for SpliceInfoSection {
    fn validate_crc(&self, original_buffer: &[u8]) -> Result<bool, io::Error> {
        crc::validate_message_crc(original_buffer)
    }
    
    fn get_crc(&self) -> u32 {
        self.crc_32
    }
}

### 6. Error Handling Options

Two approaches for handling CRC validation failures:

#### Option A: Strict Validation (Recommended)
- Parser fails with error if CRC validation fails
- Ensures data integrity by default
- Users can disable with `--no-default-features` if needed

#### Option B: Lenient Validation
- Parser succeeds but sets a validation flag
- Add `crc_valid: Option<bool>` field to `SpliceInfoSection`
- Users can check validation status after parsing

### 7. CLI Tool Enhancement

Update the CLI tool to show CRC validation status (CRC validation is always available in CLI):

```rust
#[cfg(feature = "cli")]
fn main() {
    // ... existing CLI logic ...
    
    match parse_splice_info_section(&buffer) {
        Ok(section) => {
            println!("Successfully parsed SpliceInfoSection:");
            // ... existing output ...
            
            // CRC validation is always available when CLI feature is enabled
            // since cli feature depends on crc-validation
            match validate_scte35_crc(&buffer) {
                Ok(true) => println!("  CRC-32: 0x{:08X} ✓ (Valid)", section.crc_32),
                Ok(false) => println!("  CRC-32: 0x{:08X} ✗ (Invalid)", section.crc_32),
                Err(e) => println!("  CRC-32: 0x{:08X} ✗ (Error: {})", section.crc_32, e),
            }
        }
        Err(e) => eprintln!("Error parsing SCTE-35: {}", e),
    }
}
```

**Note**: Since the `cli` feature automatically enables `crc-validation`, we don't need feature flag checking in the CLI code. CRC validation will always be available.

### 8. Documentation Updates

Update documentation to reflect CRC validation capabilities:

#### CLAUDE.md
```markdown
### Build Library with CRC Validation (Default)
```bash
cargo build
```

### Build Library without CRC Validation
```bash
cargo build --no-default-features
```

### Build CLI Tool (Always includes CRC validation)
```bash
cargo build --features cli
```

**Note**: The CLI tool automatically enables CRC validation since it needs to provide complete diagnostic information.
```

#### README.md
Add section about CRC validation:

```markdown
## CRC Validation

By default, the library validates CRC-32 checksums in SCTE-35 messages to ensure data integrity. This feature can be disabled if needed:

### With CRC Validation (Default)
```toml
[dependencies]
scte35-parsing = "0.1.0"
```

### Without CRC Validation (Library only)
```toml
[dependencies]
scte35-parsing = { version = "0.1.0", default-features = false }
```

### With CLI Tool (Automatically includes CRC validation)
```toml
[dependencies]
scte35-parsing = { version = "0.1.0", features = ["cli"] }
```

**Note**: The CLI feature automatically enables CRC validation to provide complete message diagnostics.
```

### 9. Testing

Add comprehensive tests for CRC validation:

```rust
#[cfg(test)]
#[cfg(feature = "crc-validation")]
mod crc_tests {
    use super::*;
    use base64::{engine::general_purpose, Engine};

    #[test]
    fn test_valid_crc() {
        let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = general_purpose::STANDARD.decode(valid_message).unwrap();
        
        let result = validate_scte35_crc(&buffer);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_invalid_crc() {
        let mut buffer = general_purpose::STANDARD
            .decode("/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==")
            .unwrap();
        
        // Corrupt the CRC (last 4 bytes)
        let len = buffer.len();
        buffer[len - 1] = 0x00;
        
        let result = validate_scte35_crc(&buffer);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_parse_with_crc_validation() {
        let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = general_purpose::STANDARD.decode(valid_message).unwrap();
        
        // Should parse successfully with valid CRC
        let section = parse_splice_info_section(&buffer);
        assert!(section.is_ok());
    }
}
```

## Benefits

1. **Data Integrity**: Ensures SCTE-35 messages haven't been corrupted during transmission
2. **Security**: Helps detect tampered messages
3. **Debugging**: Identifies parsing issues vs. data corruption
4. **Standards Compliance**: Follows SCTE-35 specification requirements
5. **Flexibility**: Optional feature allows users to choose performance vs. validation trade-offs

## Migration Path

1. Add the feature as optional with default enabled
2. Existing code continues to work without changes
3. Users who want to disable CRC validation can opt out
4. Future versions could make strict validation the only option

## Module Structure Summary

The CRC validation implementation follows a clean modular design:

```
src/
├── lib.rs              # Main library with re-exports
├── crc.rs              # Dedicated CRC module
└── main.rs             # CLI tool (optional)
```

### Key Design Principles

1. **Module-level isolation**: Entire `crc` module excluded when feature disabled
2. **Minimal feature flags**: Only used at module boundaries and key integration points
3. **Clean compilation**: No dead code or unused dependencies when feature disabled
4. **Simple API**: Consistent behavior whether feature is enabled or not
5. **Zero overhead**: No runtime cost when feature is disabled

### API Surface

```rust
// Top-level convenience function (always available, returns false when disabled)
use scte35_parsing::validate_scte35_crc;
let is_valid = validate_scte35_crc(&buffer)?;

// Direct module access (only available when feature enabled)
#[cfg(feature = "crc-validation")]
use scte35_parsing::crc::{validate_crc, calculate_crc, validate_message_crc};

// Method-based validation (always available, returns false when disabled)
let is_valid = section.validate_crc(&buffer)?;

// Trait-based validation (only available when feature enabled)
#[cfg(feature = "crc-validation")]
use scte35_parsing::CrcValidatable;
```

### Benefits of This Approach

1. **Cleaner code**: Much less `#[cfg(...)]` noise throughout the codebase
2. **Better maintainability**: Feature logic concentrated at module boundaries
3. **Simpler testing**: Tests in `crc.rs` don't need feature flags
4. **Zero overhead**: Entire module excluded from compilation when disabled
5. **Consistent API**: Functions always exist but return appropriate "disabled" responses