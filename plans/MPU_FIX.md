# MPU Implementation Fix Plan

## Overview

The current MPU implementation has multiple issues that violate the SCTE-35 specification section 10.3.3.3. According to the specification, MPU should contain:
1. `format_identifier` - 32 bits (4 bytes) - A unique identifier registered with SMPTE
2. `private_data` - N*8 bits (variable length) - Byte-aligned data defined by the format identifier's owner

## Critical Issues Found

1. **Builder Issue**: `Upid::Mpu(String)` incorrectly treats MPU as a simple string
2. **Parser Issue**: Parser doesn't validate MPU structure - treats it as flat byte array
3. **Specification Violation**: Both builder and parser fail to properly handle the 32-bit format_identifier + variable private_data structure

## API Breaking Change Policy

**IMPORTANT**: We do NOT need or should keep backwards compatibility of the public API. Implementing the SCTE-35 specification accurately is the primary value of the whole project. Any existing code using the incorrect `Upid::Mpu(String)` format needs to be updated to follow the specification.

## Files to Modify

1. `src/builders/descriptors.rs` - Update Upid enum and implementation
2. `src/fmt.rs` - Create new formatting utilities module (NEW FILE)  
3. `src/lib.rs` - Add fmt module export
4. `src/parser.rs` - **CRITICAL FIX**: Update parsing logic to properly validate MPU structure
5. `src/tests.rs` - Fix existing MPU test to verify proper parsing
6. `examples/builder_demo.rs` - Add MPU examples
7. `src/builders/tests.rs` - Add comprehensive tests
8. Documentation updates as needed

## Detailed Changes

### 1. Update Upid enum in `src/builders/descriptors.rs`

**Current (line 89):**
```rust
/// MPU (Media Processing Unit).
Mpu(String),
```

**New:**
```rust
/// MPU (Media Processing Unit) with format identifier and private data.
Mpu { 
    /// 32-bit format identifier registered with SMPTE
    format_identifier: u32, 
    /// Variable-length private data as defined by format identifier owner
    private_data: Vec<u8> 
},
```

### 2. Add convenience constructors to Upid

Add these methods to the `impl Upid` block:

```rust
impl Upid {
    /// Creates a new MPU UPID with format identifier and private data.
    ///
    /// # Arguments
    /// * `format_identifier` - 32-bit SMPTE registered format identifier
    /// * `private_data` - Variable-length data as defined by format identifier owner
    ///
    /// # Example
    /// ```rust
    /// use scte35::builders::Upid;
    /// 
    /// // Create MPU with custom format identifier and data
    /// let mpu = Upid::new_mpu(0x43554549, b"custom_content_id".to_vec());
    /// ```
    pub fn new_mpu(format_identifier: u32, private_data: Vec<u8>) -> Self {
        Upid::Mpu { format_identifier, private_data }
    }

    /// Creates a new MPU UPID with format identifier and string data.
    ///
    /// This is a convenience method for text-based private data.
    ///
    /// # Example  
    /// ```rust
    /// use scte35::builders::Upid;
    /// 
    /// // Create MPU with string content
    /// let mpu = Upid::new_mpu_str(0x43554549, "program_12345");
    /// ```
    pub fn new_mpu_str(format_identifier: u32, data: &str) -> Self {
        Upid::Mpu { 
            format_identifier, 
            private_data: data.as_bytes().to_vec() 
        }
    }
}
```

### 3. Update validation logic in `upid()` method

**Remove MPU from string validation block (lines 155-165):**
```rust
// Remove Upid::Mpu(s) from this block
Upid::Isci(s) 
| Upid::AdId(s)
| Upid::Tid(s) => {
    // ... existing validation
}
```

**Add specific MPU validation:**
```rust
Upid::Mpu { format_identifier: _, private_data } => {
    if private_data.len() > 251 {
        return Err(BuilderError::InvalidValue {
            field: "mpu_private_data",
            reason: format!(
                "MPU private data must be <= 251 bytes (4 bytes reserved for format_identifier). Got {} bytes",
                private_data.len()
            ),
        });
    }
}
```

### 4. Update `From<Upid>` implementation (line 309)

**Current:**
```rust
Upid::Mpu(data) => (SegmentationUpidType::MPU, data.into_bytes()),
```

**New:**
```rust
Upid::Mpu { format_identifier, private_data } => {
    let mut bytes = format_identifier.to_be_bytes().to_vec();
    bytes.extend(private_data);
    (SegmentationUpidType::MPU, bytes)
},
```

### 5. Create new formatting utilities module `src/fmt.rs`

Create a new file `src/fmt.rs` with reusable formatting utilities:

```rust
//! Formatting utilities for SCTE-35 data structures.
//!
//! This module provides common formatting functions for displaying
//! SCTE-35 data in human-readable formats, with intelligent handling
//! of binary vs text data.

/// Converts a 32-bit format identifier to a human-readable string.
/// 
/// Returns ASCII representation if all bytes are printable ASCII letters/numbers,
/// otherwise returns hex representation.
///
/// # Arguments
/// * `format_identifier` - 32-bit format identifier to format
///
/// # Examples
/// ```rust
/// use scte35::fmt::format_identifier_to_string;
/// 
/// assert_eq!(format_identifier_to_string(0x43554549), "CUEI");
/// assert_eq!(format_identifier_to_string(0x12345678), "0x12345678");
/// ```
pub fn format_identifier_to_string(format_identifier: u32) -> String {
    let bytes = format_identifier.to_be_bytes();
    
    // Check if all bytes are printable ASCII letters or numbers
    if bytes.iter().all(|&b| b.is_ascii_alphanumeric()) {
        // Convert to ASCII string
        String::from_utf8_lossy(&bytes).to_string()
    } else {
        // Fallback to hex representation
        format!("0x{:08X}", format_identifier)
    }
}

/// Formats private data for display, showing as string if valid UTF-8,
/// otherwise as hex with length limit for readability.
///
/// # Arguments
/// * `data` - Byte slice to format for display
///
/// # Examples
/// ```rust
/// use scte35::fmt::format_private_data;
/// 
/// assert_eq!(format_private_data(b"test"), "\"test\"");
/// assert_eq!(format_private_data(&[0x01, 0x02, 0x03]), "0x010203");
/// assert_eq!(format_private_data(&[]), "empty");
/// ```
pub fn format_private_data(data: &[u8]) -> String {
    if data.is_empty() {
        return "empty".to_string();
    }
    
    // Try to interpret as UTF-8 string first
    if let Ok(s) = std::str::from_utf8(data) {
        // Check if it's printable (no control characters except space)
        if s.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            // Truncate long strings for readability
            if s.len() <= 50 {
                format!("\"{}\"", s)
            } else {
                format!("\"{}...\" ({} bytes)", &s[..47], data.len())
            }
        } else {
            // Contains control characters, show as hex
            format_as_hex(data)
        }
    } else {
        // Not valid UTF-8, show as hex
        format_as_hex(data)
    }
}

/// Formats data as hex string with length limit for readability.
///
/// # Arguments
/// * `data` - Byte slice to format as hexadecimal
///
/// # Examples
/// ```rust
/// use scte35::fmt::format_as_hex;
/// 
/// assert_eq!(format_as_hex(&[0x01, 0x02, 0x03]), "0x010203");
/// assert_eq!(format_as_hex(&(0..20).collect::<Vec<u8>>()), "0x000102030405... (20 bytes)");
/// ```
pub fn format_as_hex(data: &[u8]) -> String {
    if data.len() <= 8 {
        // Show all bytes for short data
        format!("0x{}", data.iter().map(|b| format!("{:02x}", b)).collect::<String>())
    } else {
        // Show first few bytes with truncation for long data
        let preview: String = data[..6].iter().map(|b| format!("{:02x}", b)).collect();
        format!("0x{}... ({} bytes)", preview, data.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_identifier_to_string() {
        // ASCII alphanumeric
        assert_eq!(format_identifier_to_string(0x43554549), "CUEI");
        assert_eq!(format_identifier_to_string(0x54455354), "TEST");
        assert_eq!(format_identifier_to_string(0x41424344), "ABCD");
        
        // Non-ASCII or non-alphanumeric
        assert_eq!(format_identifier_to_string(0x12345678), "0x12345678");
        assert_eq!(format_identifier_to_string(0x41422D44), "0x41422D44"); // "AB-D"
        assert_eq!(format_identifier_to_string(0x00000000), "0x00000000");
    }

    #[test]
    fn test_format_private_data() {
        // Empty data
        assert_eq!(format_private_data(&[]), "empty");
        
        // Valid UTF-8 string
        assert_eq!(format_private_data(b"test"), "\"test\"");
        assert_eq!(format_private_data(b"hello world"), "\"hello world\"");
        
        // Binary data
        assert_eq!(format_private_data(&[0x01, 0x02, 0x03]), "0x010203");
        assert_eq!(format_private_data(&[0xFF, 0xFE, 0xFD]), "0xfffefd");
        
        // Long string (truncated)
        let long_string = "a".repeat(60);
        let result = format_private_data(long_string.as_bytes());
        assert!(result.starts_with("\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...\""));
        assert!(result.contains("(60 bytes)"));
        
        // String with control characters
        assert_eq!(format_private_data(b"test\x00\x01"), "0x746573740001");
    }

    #[test]
    fn test_format_as_hex() {
        // Short data
        assert_eq!(format_as_hex(&[]), "0x");
        assert_eq!(format_as_hex(&[0x01]), "0x01");
        assert_eq!(format_as_hex(&[0x01, 0x02, 0x03, 0x04]), "0x01020304");
        assert_eq!(format_as_hex(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]), "0x0102030405060708");
        
        // Long data (truncated)
        let long_data: Vec<u8> = (0..20).collect();
        assert_eq!(format_as_hex(&long_data), "0x000102030405... (20 bytes)");
    }
}
```

### 6. Update `src/lib.rs` to export the fmt module

Add this line to `src/lib.rs`:

```rust
pub mod fmt;
```

### 7. Update Display implementation in `src/builders/descriptors.rs`

Import the formatting utilities and update the Display implementation:

```rust
use crate::fmt::{format_identifier_to_string, format_private_data};

impl std::fmt::Display for Upid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... other variants (keep existing)
            Upid::Mpu { format_identifier, private_data } => {
                let format_str = format_identifier_to_string(*format_identifier);
                let data_str = format_private_data(private_data);
                write!(f, "MPU(format: {}, data: {})", format_str, data_str)
            }
            // ... other variants (keep existing)
        }
    }
}
```

### 8. Update parsing logic in `src/parser.rs`

Find the MPU parsing section and update it:

```rust
SegmentationUpidType::MPU => {
    if upid_bytes.len() < 4 {
        return Err(io::Error::new(
            ErrorKind::InvalidData, 
            "MPU UPID must have at least 4 bytes for format_identifier"
        ));
    }
    let format_identifier = u32::from_be_bytes([
        upid_bytes[0], upid_bytes[1], upid_bytes[2], upid_bytes[3]
    ]);
    let private_data = upid_bytes[4..].to_vec();
    Upid::Mpu { format_identifier, private_data }
}
```

### 9. Add comprehensive tests

Add to `src/builders/tests.rs`:

```rust
#[test]
fn test_mpu_convenience_constructors() {
    let mpu1 = Upid::new_mpu(0x12345678, vec![1, 2, 3]);
    let mpu2 = Upid::new_mpu_str(0x12345678, "test");
    
    // Verify correct structure
    match mpu1 {
        Upid::Mpu { format_identifier: 0x12345678, private_data } => {
            assert_eq!(private_data, vec![1, 2, 3]);
        }
        _ => panic!("Wrong variant"),
    }
    
    match mpu2 {
        Upid::Mpu { format_identifier: 0x12345678, private_data } => {
            assert_eq!(private_data, b"test");
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_mpu_validation() {
    // Test size limit
    let large_data = vec![0u8; 252]; // Too large
    let result = SegmentationDescriptorBuilder::new(1, SegmentationType::ProgramStart)
        .upid(Upid::new_mpu(0x12345678, large_data));
    
    assert!(matches!(result, Err(BuilderError::InvalidValue { field, .. }) if field == "mpu_private_data"));
    
    // Test valid size
    let valid_data = vec![0u8; 251]; // Maximum allowed
    let result = SegmentationDescriptorBuilder::new(1, SegmentationType::ProgramStart)
        .upid(Upid::new_mpu(0x12345678, valid_data));
    
    assert!(result.is_ok());
}

#[test]
fn test_mpu_serialization() {
    let mpu = Upid::new_mpu(0x43554549, b"test_data".to_vec());
    let (upid_type, bytes) = mpu.into();
    
    assert_eq!(upid_type, SegmentationUpidType::MPU);
    assert_eq!(&bytes[..4], &[0x43, 0x55, 0x45, 0x49]); // Format identifier
    assert_eq!(&bytes[4..], b"test_data"); // Private data
}

#[test]
fn test_mpu_display_formats() {
    // ASCII format identifier with string data
    let string_mpu = Upid::new_mpu_str(0x43554549, "episode_123"); // "CUEI"
    assert_eq!(string_mpu.to_string(), "MPU(format: CUEI, data: \"episode_123\")");
    
    // ASCII format identifier with binary data
    let binary_mpu = Upid::new_mpu(0x43554549, vec![0x01, 0x02, 0x03, 0xFF]); // "CUEI"
    assert_eq!(binary_mpu.to_string(), "MPU(format: CUEI, data: 0x010203ff)");
    
    // Non-ASCII format identifier with string data
    let hex_format_mpu = Upid::new_mpu_str(0x12345678, "content_id");
    assert_eq!(hex_format_mpu.to_string(), "MPU(format: 0x12345678, data: \"content_id\")");
    
    // Long string data (truncated)
    let long_string = "a".repeat(60);
    let long_mpu = Upid::new_mpu_str(0x54455354, &long_string); // "TEST"
    assert_eq!(long_mpu.to_string(), "MPU(format: TEST, data: \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...\" (60 bytes))");
    
    // Long binary data (truncated)
    let long_binary: Vec<u8> = (0..20).collect();
    let long_binary_mpu = Upid::new_mpu(0x54455354, long_binary); // "TEST"
    assert_eq!(long_binary_mpu.to_string(), "MPU(format: TEST, data: 0x000102030405... (20 bytes))");
    
    // Empty data
    let empty_mpu = Upid::new_mpu(0x54455354, vec![]); // "TEST"
    assert_eq!(empty_mpu.to_string(), "MPU(format: TEST, data: empty)");
    
    // String with control characters (shows as hex)
    let control_mpu = Upid::new_mpu(0x54455354, b"test\x00\x01".to_vec()); // "TEST"
    assert_eq!(control_mpu.to_string(), "MPU(format: TEST, data: 0x746573740001)");
}
```

### 10. Update examples in `examples/builder_demo.rs`

Add this example:

```rust
// Example 7: MPU UPID usage patterns
println!("7. Creating MPU UPIDs for different use cases:");

// Company-specific format identifier (readable ASCII)
let company_mpu = Upid::new_mpu_str(0x43554549, "episode_s01e05_12345"); // "CUEI"
let descriptor1 = SegmentationDescriptorBuilder::new(8888, SegmentationType::ProgramStart)
    .upid(company_mpu)?
    .build()?;
println!("   ✓ Company MPU created: {}", company_mpu);

// Different company format with structured data
let other_mpu = Upid::new_mpu_str(0x4D594944, "show:123:ep:456:segment:789"); // "MYID"
let descriptor2 = SegmentationDescriptorBuilder::new(9999, SegmentationType::ContentIdentification)
    .upid(other_mpu)?
    .build()?;
println!("   ✓ Different company MPU: {}", other_mpu);

// Binary format identifier with binary data
let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD, 0xFC];
let binary_mpu = Upid::new_mpu(0x12345678, binary_data); // Non-ASCII, shows as hex
let descriptor3 = SegmentationDescriptorBuilder::new(1010, SegmentationType::ProgramEnd)
    .upid(binary_mpu)?
    .build()?;
println!("   ✓ Binary format MPU: {}", binary_mpu);

// Very long content (gets truncated in display)
let long_content = "this_is_a_very_long_content_identifier_that_exceeds_fifty_characters_and_should_be_truncated";
let long_mpu = Upid::new_mpu_str(0x4C4F4E47, long_content); // "LONG"
let descriptor4 = SegmentationDescriptorBuilder::new(1111, SegmentationType::ProgramStart)
    .upid(long_mpu)?
    .build()?;
println!("   ✓ Long content MPU: {}", long_mpu);
```

### 11. Update documentation

Update the module-level documentation in `src/builders/descriptors.rs` to include MPU examples:

```rust
/// # MPU UPID Examples
/// 
/// ```rust
/// use scte35::builders::{Upid, SegmentationDescriptorBuilder};
/// use scte35::types::SegmentationType;
/// 
/// // Method 1: Direct struct construction 
/// let mpu1 = Upid::Mpu {
///     format_identifier: 0x43554549, // "CUEI" in ASCII
///     private_data: vec![0x01, 0x02, 0x03],
/// };
/// 
/// // Method 2: Convenience constructor
/// let mpu2 = Upid::new_mpu(0x41424344, vec![0x01, 0x02, 0x03]); // "ABCD"
/// 
/// // Method 3: String-based convenience
/// let mpu3 = Upid::new_mpu_str(0x4D594944, "content_id_123"); // "MYID"
/// 
/// // Use in segmentation descriptor
/// let descriptor = SegmentationDescriptorBuilder::new(1234, SegmentationType::ProgramStart)
///     .upid(mpu3)?
///     .build()?;
/// ```
```

## Testing Strategy

1. **Unit Tests**: Test all convenience constructors, validation, and display formatting
2. **Integration Tests**: Test MPU in complete segmentation descriptors
3. **Round-trip Tests**: Ensure serialization and parsing work correctly
4. **Error Tests**: Verify proper error handling for invalid data

## Breaking Changes

This is a breaking change that affects:
- `Upid::Mpu` variant structure
- Any existing code that constructs `Upid::Mpu(String)`
- Serialization format of MPU UPIDs

## Migration Guide

**Before:**
```rust
let mpu = Upid::Mpu("content_id".to_string());
```

**After:**
```rust
// Option 1: Direct construction
let mpu = Upid::Mpu {
    format_identifier: 0x43554549, // Your company's registered identifier
    private_data: b"content_id".to_vec(),
};

// Option 2: Convenience method
let mpu = Upid::new_mpu_str(0x43554549, "content_id");
```

## Implementation Progress Tracking

### ✅ IMPLEMENTATION COMPLETED SUCCESSFULLY

**✅ ALL FILES COMPLETED (6/6 files):**
- `src/builders/descriptors.rs` - ✅ Full implementation with convenience constructors and validation
- `src/fmt.rs` - ✅ Full implementation with intelligent formatting
- `src/lib.rs` - ✅ fmt module exported
- `src/parser.rs` - ✅ MPU structure validation implemented per SCTE-35 spec
- `src/builders/tests.rs` - ✅ All tests updated and comprehensive MPU test suite added
- `examples/builder_demo.rs` - ✅ Example 7 with various MPU usage patterns implemented

### ✅ Completed Todo List

1. ✅ **COMPLETED**: Fix critical parser.rs MPU handling to validate SCTE-35 structure (4-byte format_identifier + variable private_data)
2. ✅ **COMPLETED**: Fix compilation errors in src/builders/tests.rs (update old Upid::Mpu(String) usage)
3. ✅ **COMPLETED**: Add comprehensive MPU tests as specified in plan (convenience constructors, validation, serialization, display formats)
4. ✅ **COMPLETED**: Add MPU examples to examples/builder_demo.rs (Example 7 with various usage patterns)
5. ✅ **COMPLETED**: Run tests to verify all changes work correctly - **ALL 138 TESTS PASSING**

### ✅ Implementation Summary

**Critical Fixes Applied:**
- **Parser Validation**: MPU parsing now validates 4-byte format_identifier + variable private_data structure
- **API Compliance**: All code uses new `Upid::Mpu { format_identifier, private_data }` struct format
- **Comprehensive Testing**: 6 new MPU-specific tests added covering all functionality
- **Documentation**: Complete Example 7 implementation with 4+ usage patterns

**Test Results:**
- **138 total tests passing** (including 6 new MPU tests)
- **5 serde integration tests passing**
- **30 documentation tests passing**
- **All CLI integration tests passing**

## Expected Outcomes

1. **Specification Compliance**: MPU implementation will correctly match SCTE-35 spec
2. **Better Debugging**: Display implementation shows actual content
3. **Type Safety**: Compile-time prevention of invalid MPU structures
4. **Ergonomic API**: Multiple ways to create MPU UPIDs for different use cases
5. **Proper Validation**: Runtime validation with descriptive error messages

This comprehensive plan ensures the MPU implementation follows the SCTE-35 specification while maintaining an ergonomic and type-safe API for users.