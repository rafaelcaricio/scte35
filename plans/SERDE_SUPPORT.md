# SCTE-35 Serde Support Implementation Plan

## Overview

This document outlines the plan for adding serde serialization/deserialization support to the scte35-parsing library. The serde support will be implemented as an optional feature flag (`serde`) that is enabled by default, allowing users to serialize parsed SCTE-35 messages to JSON and other serde-supported formats.

## Design Goals

1. **Feature Flag**: Implement serde support as an optional feature flag (`serde`) that is enabled by default
2. **Minimal Code Intrusion**: Concentrate serde-specific code in a new `src/serde.rs` module
3. **Preserve API**: The serde implementation should be purely additive and not break existing APIs
4. **Comprehensive Coverage**: Support serialization for all public types
5. **Sensible Defaults**: Provide good default serialization behavior while allowing customization

## Implementation Strategy

### 1. Update Cargo.toml

```toml
[features]
default = ["crc-validation", "serde"]
crc-validation = ["crc"]
cli = ["base64", "crc-validation"]
serde = ["dep:serde", "dep:serde_json", "base64"]

[dependencies]
crc = { version = "3.0", optional = true }
base64 = { version = "0.22.1", optional = true }
serde = { version = "1.0", features = ["derive"], optional = true }
serde_json = { version = "1.0", optional = true }
```

**Note**: We include `base64` with the `serde` feature to enable base64 encoding of binary data in JSON output.

### 2. Module Structure

Create a new module `src/serde.rs` that will contain:
- Custom serialize/deserialize implementations for complex types
- Helper functions for binary data encoding
- Trait implementations for types that need special handling

### 3. Types Requiring Serde Support

#### Core Types (src/types.rs)
- `SpliceInfoSection` - Main structure, derive Serialize/Deserialize
- `SpliceCommand` (enum) - Derive with serde enum representation
- `SpliceNull` - Simple derive
- `SpliceSchedule` - Derive
- `SpliceInsert` - Derive
- `TimeSignal` - Derive
- `BandwidthReservation` - Derive
- `PrivateCommand` - Needs custom serialization for `private_bytes`
- `ComponentSplice` - Derive
- `SpliceInsertComponent` - Derive
- `SegmentationType` (enum) - Custom serialization to include both ID and description

#### Time Types (src/time.rs)
- `SpliceTime` - Derive with additional computed field for duration
- `DateTime` - Derive
- `BreakDuration` - Derive with additional computed field for duration

#### Descriptor Types (src/descriptors.rs)
- `SpliceDescriptor` (enum) - Custom serialization for better structure
- `SegmentationDescriptor` - Custom serialization to include computed fields

#### UPID Types (src/upid.rs)
- `SegmentationUpidType` (enum) - Custom serialization to include both value and description

### 4. Special Serialization Requirements

#### Binary Data Fields
Fields containing raw bytes (`Vec<u8>`) should be serialized as base64-encoded strings:
- `PrivateCommand::private_bytes`
- `SegmentationDescriptor::segmentation_upid`
- `SpliceDescriptor::Unknown::data`
- `SpliceInfoSection::alignment_stuffing_bits`

#### Time Fields
PTS time values (90kHz ticks) should include both raw and human-readable representations:
- Include raw tick value as-is
- Add computed duration field in a human-readable format (e.g., seconds or ISO duration)

#### Enum Representations
- `SpliceCommand`: Use internally tagged representation with "type" field
- `SegmentationType`: Serialize as object with "id" and "description" fields
- `SegmentationUpidType`: Serialize as object with "value" and "description" fields

#### Computed/Derived Fields
Include helpful computed fields in serialization:
- `SegmentationDescriptor`: Include UPID as human-readable string when possible
- Time durations: Include both ticks and seconds/duration representation

### 5. Implementation Details

#### src/lib.rs Changes
```rust
#[cfg(feature = "serde")]
mod serde;

// Add feature-gated derive macros to existing types
```

#### src/serde.rs Structure
```rust
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use serde::de::{self, Visitor};
use base64::{Engine, engine::general_purpose};

// Custom serializers for binary data
pub fn serialize_bytes<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&general_purpose::STANDARD.encode(bytes))
}

pub fn deserialize_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    // Implementation
}

// Custom implementations for special types
impl Serialize for SegmentationType {
    // Include both ID and description
}

impl Serialize for SegmentationDescriptor {
    // Include computed fields like upid_as_string()
}

// Helper structures for enhanced serialization
#[derive(Serialize, Deserialize)]
struct DurationInfo {
    ticks: u64,
    seconds: f64,
    iso_duration: String,
}
```

### 6. Conditional Compilation Strategy

Use `#[cfg_attr]` to conditionally add serde derives:

```rust
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpliceInfoSection {
    // fields...
}
```

For types needing custom serialization, implement the traits in the serde module and use trait bounds.

### 7. Testing Strategy

Create comprehensive tests in `src/serde.rs` (module-level tests) and integration tests:
- Round-trip serialization tests (serialize -> deserialize -> compare)
- JSON output format validation
- Binary data encoding verification
- Edge case handling (None values, empty vecs, etc.)

### 8. Documentation Updates

- Update README.md with serde usage examples
- Add module-level documentation to src/serde.rs
- Include JSON output examples in documentation
- Document how to disable serde feature if needed

## Migration Path

Since serde is enabled by default, existing users will automatically get serialization support. Users who want to opt-out can use:

```toml
[dependencies]
scte35-parsing = { version = "0.1.0", default-features = false, features = ["crc-validation"] }
```

## Example Usage

After implementation, users will be able to:

```rust
use scte35_parsing::parse_splice_info_section;
use serde_json;

let base64_payload = "...";
let data = base64::decode(base64_payload).unwrap();
let splice_info = parse_splice_info_section(&data).unwrap();

// Serialize to JSON
let json = serde_json::to_string_pretty(&splice_info).unwrap();
println!("{}", json);

// Deserialize from JSON
let deserialized: SpliceInfoSection = serde_json::from_str(&json).unwrap();
```

## Implementation Order

1. Add serde dependencies to Cargo.toml
2. Create src/serde.rs with helper functions
3. Add conditional derive attributes to simple types
4. Implement custom serialization for complex types
5. Add comprehensive tests
6. Update documentation
7. Update CLI tool to optionally output JSON

## Future Enhancements

- Custom serde formats (compact vs. verbose)
- YAML support demonstration
- Binary formats (MessagePack, CBOR)
- Schema generation for JSON output