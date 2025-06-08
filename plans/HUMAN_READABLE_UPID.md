# Human Readable UPID Types Implementation Plan

## Overview

This plan outlines the implementation of human-readable UPID (Unique Program Identifier) types in the SCTE-35 parsing library. UPIDs are found within segmentation descriptors and provide standardized identifiers for content segments.

## Current State Analysis

### Existing Infrastructure
- ✅ Basic descriptor parsing (stores raw bytes in `SpliceDescriptor`)
- ✅ Test cases for various UPID types (UMID, ISAN, AIRID, AdID)
- ✅ Documentation structure and standards
- ✅ CLI tool for displaying parsed information
- ✅ CRC validation system
- ✅ Comprehensive test coverage

### Current Limitations
- ❌ Segmentation descriptors are not parsed beyond raw bytes
- ❌ UPID types are not identified or decoded
- ❌ CLI doesn't display human-readable UPID information
- ❌ No structured access to UPID data for library users

## Implementation Strategy

### Phase 1: Core UPID Type System
1. **Add SegmentationUpidType enum** with all standard UPID types
2. **Implement From<u8> trait** for UPID type identification
3. **Add Into<u8> trait** for serialization support
4. **Create UPID-specific data structures** for typed access

### Phase 2: Segmentation Descriptor Parsing
1. **Create SegmentationDescriptor struct** with parsed fields
2. **Implement segmentation descriptor parsing logic**
3. **Add UPID parsing within segmentation descriptors**
4. **Extend SpliceDescriptor enum** to support typed descriptors

### Phase 3: Enhanced API and CLI
1. **Update CLI tool** to display human-readable UPID information
2. **Add convenience methods** for UPID access
3. **Implement Display traits** for human-readable output
4. **Add UPID validation** where applicable

### Phase 4: Testing and Documentation
1. **Create comprehensive test suite** for all UPID types
2. **Update documentation** with UPID examples
3. **Add doctests** for new functionality
4. **Update README.md** with UPID features

## Detailed Implementation Plan

### 1. SegmentationUpidType Enum

```rust
/// Represents the different types of UPIDs (Unique Program Identifiers) used in segmentation descriptors.
///
/// UPIDs provide standardized ways to identify content segments for various purposes
/// including ad insertion, content identification, and distribution control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SegmentationUpidType {
    /// No UPID is used (0x00)
    NotUsed,
    /// User-defined UPID (deprecated) (0x01)
    UserDefinedDeprecated,
    /// ISCI (Industry Standard Commercial Identifier) (0x02)
    ISCI,
    /// Ad Identifier (0x03)
    AdID,
    /// UMID (Unique Material Identifier) (0x04)
    UMID,
    /// ISAN (International Standard Audiovisual Number) - deprecated (0x05)
    ISANDeprecated,
    /// ISAN (International Standard Audiovisual Number) (0x06)
    ISAN,
    /// TID (Turner Identifier) (0x07)
    TID,
    /// AiringID (0x08)
    AiringID,
    /// ADI (Advertising Digital Identification) (0x09)
    ADI,
    /// EIDR (Entertainment Identifier Registry) (0x0A)
    EIDR,
    /// ATSC Content Identifier (0x0B)
    ATSCContentIdentifier,
    /// MPU (Media Processing Unit) (0x0C)
    MPU,
    /// MID (Media Identifier) (0x0D)
    MID,
    /// ADS Information (0x0E)
    ADSInformation,
    /// URI (Uniform Resource Identifier) (0x0F)
    URI,
    /// UUID (Universally Unique Identifier) (0x10)
    UUID,
    /// SCR (Subscriber Company Reporting) (0x11)
    SCR,
    /// Reserved or unknown UPID type
    Reserved(u8),
}
```

### 2. Segmentation Descriptor Structure

```rust
/// Represents a parsed segmentation descriptor (tag 0x02).
///
/// Segmentation descriptors provide detailed information about content segments,
/// including timing, UPID data, and segmentation types.
#[derive(Debug, Clone)]
pub struct SegmentationDescriptor {
    /// Segmentation event identifier
    pub segmentation_event_id: u32,
    /// Indicates if this event should be cancelled
    pub segmentation_event_cancel_indicator: bool,
    /// Program segmentation flag
    pub program_segmentation_flag: bool,
    /// Segmentation duration flag
    pub segmentation_duration_flag: bool,
    /// Delivery not restricted flag
    pub delivery_not_restricted_flag: bool,
    /// Web delivery allowed flag
    pub web_delivery_allowed_flag: Option<bool>,
    /// No regional blackout flag
    pub no_regional_blackout_flag: Option<bool>,
    /// Archive allowed flag
    pub archive_allowed_flag: Option<bool>,
    /// Device restrictions
    pub device_restrictions: Option<u8>,
    /// Segmentation duration in 90kHz ticks
    pub segmentation_duration: Option<u64>,
    /// UPID type identifier
    pub segmentation_upid_type: SegmentationUpidType,
    /// Length of UPID data
    pub segmentation_upid_length: u8,
    /// Raw UPID data bytes
    pub segmentation_upid: Vec<u8>,
    /// Segmentation type identifier
    pub segmentation_type_id: u8,
    /// Segment number
    pub segment_num: u8,
    /// Expected number of segments
    pub segments_expected: u8,
    /// Sub-segment number (for some segmentation types)
    pub sub_segment_num: Option<u8>,
    /// Expected number of sub-segments
    pub sub_segments_expected: Option<u8>,
}
```

### 3. Enhanced SpliceDescriptor Enum

```rust
/// Represents different types of splice descriptors with parsed content.
#[derive(Debug, Clone)]
pub enum SpliceDescriptor {
    /// Avail descriptor (tag 0x00)
    Avail {
        /// Raw descriptor bytes
        data: Vec<u8>,
    },
    /// DTMF descriptor (tag 0x01)
    DTMF {
        /// Raw descriptor bytes
        data: Vec<u8>,
    },
    /// Segmentation descriptor (tag 0x02) - fully parsed
    Segmentation(SegmentationDescriptor),
    /// Time descriptor (tag 0x03)
    Time {
        /// Raw descriptor bytes
        data: Vec<u8>,
    },
    /// Audio descriptor (tag 0x04)
    Audio {
        /// Raw descriptor bytes
        data: Vec<u8>,
    },
    /// Unknown or unsupported descriptor type
    Unknown {
        /// Descriptor tag
        tag: u8,
        /// Raw descriptor bytes
        data: Vec<u8>,
    },
}
```

### 4. UPID-Specific Parsing and Display

```rust
impl SegmentationDescriptor {
    /// Returns the UPID as a human-readable string if possible.
    pub fn upid_as_string(&self) -> Option<String> {
        match self.segmentation_upid_type {
            SegmentationUpidType::URI | SegmentationUpidType::MPU | SegmentationUpidType::AdID => {
                std::str::from_utf8(&self.segmentation_upid)
                    .ok()
                    .map(|s| s.to_string())
            }
            SegmentationUpidType::UUID => {
                if self.segmentation_upid.len() == 16 {
                    Some(format_uuid(&self.segmentation_upid))
                } else {
                    None
                }
            }
            SegmentationUpidType::ISAN => {
                if self.segmentation_upid.len() >= 12 {
                    Some(format_isan(&self.segmentation_upid))
                } else {
                    None
                }
            }
            // Add more type-specific formatting
            _ => None,
        }
    }

    /// Returns a description of the UPID type.
    pub fn upid_type_description(&self) -> &'static str {
        match self.segmentation_upid_type {
            SegmentationUpidType::NotUsed => "Not Used",
            SegmentationUpidType::UserDefinedDeprecated => "User Defined (Deprecated)",
            SegmentationUpidType::ISCI => "ISCI (Industry Standard Commercial Identifier)",
            SegmentationUpidType::AdID => "Ad Identifier",
            SegmentationUpidType::UMID => "UMID (Unique Material Identifier)",
            SegmentationUpidType::ISANDeprecated => "ISAN (Deprecated)",
            SegmentationUpidType::ISAN => "ISAN (International Standard Audiovisual Number)",
            SegmentationUpidType::TID => "TID (Turner Identifier)",
            SegmentationUpidType::AiringID => "Airing ID",
            SegmentationUpidType::ADI => "ADI (Advertising Digital Identification)",
            SegmentationUpidType::EIDR => "EIDR (Entertainment Identifier Registry)",
            SegmentationUpidType::ATSCContentIdentifier => "ATSC Content Identifier",
            SegmentationUpidType::MPU => "MPU (Media Processing Unit)",
            SegmentationUpidType::MID => "MID (Media Identifier)",
            SegmentationUpidType::ADSInformation => "ADS Information",
            SegmentationUpidType::URI => "URI (Uniform Resource Identifier)",
            SegmentationUpidType::UUID => "UUID (Universally Unique Identifier)",
            SegmentationUpidType::SCR => "SCR (Subscriber Company Reporting)",
            SegmentationUpidType::Reserved(_) => "Reserved/Unknown",
        }
    }
}
```

### 5. CLI Enhancements

Update the CLI tool in `main.rs` to display UPID information:

```rust
// In the descriptor display section:
for descriptor in &section.splice_descriptors {
    match descriptor {
        SpliceDescriptor::Segmentation(seg_desc) => {
            println!("    Segmentation Descriptor:");
            println!("      Event ID: 0x{:08x}", seg_desc.segmentation_event_id);
            println!("      UPID Type: {} (0x{:02x})", 
                     seg_desc.upid_type_description(),
                     u8::from(seg_desc.segmentation_upid_type));
            
            if let Some(upid_str) = seg_desc.upid_as_string() {
                println!("      UPID: {}", upid_str);
            } else {
                println!("      UPID (hex): {}", hex::encode(&seg_desc.segmentation_upid));
            }
            
            if let Some(duration) = seg_desc.segmentation_duration {
                println!("      Duration: {:.3} seconds", duration as f64 / 90000.0);
            }
        }
        _ => {
            println!("    Descriptor Tag: {}", descriptor.tag());
            println!("    Descriptor Length: {}", descriptor.length());
        }
    }
}
```

### 6. Backward Compatibility Strategy

To maintain backward compatibility:

1. **Keep existing SpliceDescriptor struct** as a compatibility type
2. **Add new SpliceDescriptorParsed enum** for enhanced parsing
3. **Provide conversion methods** between old and new types
4. **Use feature flags** to optionally enable enhanced parsing

```rust
/// Configuration for descriptor parsing behavior
#[derive(Debug, Clone)]
pub struct DescriptorParsingConfig {
    /// Parse segmentation descriptors into structured format
    pub parse_segmentation: bool,
    /// Parse other descriptor types (future expansion)
    pub parse_avail: bool,
    pub parse_dtmf: bool,
}

impl Default for DescriptorParsingConfig {
    fn default() -> Self {
        Self {
            parse_segmentation: true,
            parse_avail: false,
            parse_dtmf: false,
        }
    }
}
```

## Testing Strategy

### 1. Unit Tests for UPID Types
- Test all UPID type conversions (u8 ↔ SegmentationUpidType)
- Test UPID type descriptions
- Test reserved/unknown type handling

### 2. Segmentation Descriptor Parsing Tests
- Test parsing of real segmentation descriptors from existing test cases
- Test each UPID type with appropriate test data
- Test malformed descriptor handling

### 3. Integration Tests
- Update existing tests to work with new descriptor parsing
- Test CLI output with UPID information
- Test backward compatibility

### 4. Property-Based Testing
- Test round-trip parsing/serialization
- Test with random valid UPID data
- Test boundary conditions

## Documentation Requirements

### 1. API Documentation
- Comprehensive rustdoc for all new types
- Examples for common UPID types
- Migration guide for existing users

### 2. README Updates
- Add UPID parsing to feature list
- Include examples of UPID output
- Document CLI enhancements

### 3. Architecture Documentation
- Update CLAUDE.md with new commands and architecture
- Document UPID type system
- Include parsing configuration options

## Performance Considerations

### 1. Parsing Overhead
- Segmentation descriptor parsing adds minimal overhead
- UPID string conversion is done on-demand
- Raw bytes still available for performance-critical applications

### 2. Memory Usage
- Structured parsing uses slightly more memory
- Optional parsing configuration allows optimization
- String conversions are lazy and cached where beneficial

## Future Extensibility

### 1. Additional Descriptor Types
- Framework supports easy addition of other descriptor types
- Avail descriptor parsing can be added similarly
- DTMF descriptor parsing for audio applications

### 2. UPID Validation
- Add validation for specific UPID formats (UUID, ISAN, etc.)
- Implement checksum validation where applicable
- Add UPID normalization methods

### 3. Serialization Support
- Optional serde support for all new types
- JSON/YAML export of parsed UPID data
- Integration with external UPID databases

## Implementation Timeline

### Week 1: Core Types and Parsing
- Implement SegmentationUpidType enum
- Create SegmentationDescriptor struct
- Basic segmentation descriptor parsing

### Week 2: Enhanced API and Display
- Implement UPID string conversion methods
- Add human-readable display functions
- Create descriptor type enum

### Week 3: CLI and Integration
- Update CLI tool with UPID display
- Integrate with existing parsing pipeline
- Backward compatibility testing

### Week 4: Testing and Documentation
- Comprehensive test suite
- Documentation updates
- Performance optimization

## Risk Mitigation

### 1. Backward Compatibility
- **Risk**: Breaking existing API
- **Mitigation**: Keep old API intact, add new optional features
- **Testing**: Comprehensive compatibility test suite

### 2. Performance Regression
- **Risk**: Slower parsing due to additional structure
- **Mitigation**: Optional parsing, benchmarking, lazy evaluation
- **Testing**: Performance benchmarks before/after

### 3. Incomplete UPID Support
- **Risk**: Missing or incorrect UPID type handling
- **Mitigation**: Extensive real-world test data, community feedback
- **Testing**: Test with all known UPID types from industry examples

## Success Criteria

1. ✅ All existing tests pass without modification
2. ✅ New UPID parsing correctly identifies and displays all standard types
3. ✅ CLI tool shows human-readable UPID information
4. ✅ Performance impact < 5% for typical SCTE-35 messages
5. ✅ Documentation coverage 100% for new functionality
6. ✅ Test coverage > 95% for new code
7. ✅ Zero breaking changes to existing public API

## Conclusion

This implementation plan provides a comprehensive approach to adding human-readable UPID support while maintaining the library's existing strengths in performance, reliability, and ease of use. The phased approach allows for incremental development and testing, minimizing risk while delivering valuable new functionality.