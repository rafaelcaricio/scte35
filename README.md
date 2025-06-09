# SCTE-35 Parsing Library

A Rust library for parsing SCTE-35 (Society of Cable Telecommunications Engineers) messages with built-in CRC validation. SCTE-35 is a standard for inserting cue messages into video streams, commonly used for ad insertion points in broadcast television.

## Features

- **Builder Pattern API** - Type-safe builder pattern for creating SCTE-35 messages from scratch with validation
- **Serde support** - Serialize/deserialize SCTE-35 messages to/from JSON and other formats (enabled by default)
- **CRC validation** - Built-in CRC-32 validation using MPEG-2 algorithm (enabled by default)
- **Human-readable UPID parsing** - Full support for 18 standard UPID types with intelligent formatting
- **Human-readable segmentation types** - Complete set of 48 standard segmentation types with descriptive names
- **Segmentation descriptor parsing** - Complete parsing of segmentation descriptors including UPID data
- **Minimal dependencies** - Only the `crc` crate for validation (optional) and `serde` for serialization (optional)
- **Full SCTE-35 parsing** - Supports all major SCTE-35 command types
- **Bit-level precision** - Accurate parsing of bit-packed SCTE-35 messages
- **Optional CLI tool** - Command-line interface for parsing base64-encoded messages with text and JSON output formats
- **Type-safe** - Strongly typed representations of all SCTE-35 structures
- **Data integrity** - Detects corrupted or tampered SCTE-35 messages

## Installation

### With All Features (Default)

Add this to your `Cargo.toml`:

```toml
[dependencies]
scte35-parsing = "0.1.0"
```

This includes both CRC validation and serde support.

### Without Serde Support

If you don't need JSON serialization:

```toml
[dependencies]
scte35-parsing = { version = "0.1.0", default-features = false, features = ["crc-validation"] }
```

### Minimal (No CRC or Serde)

For a minimal library without CRC validation or serde:

```toml
[dependencies]
scte35-parsing = { version = "0.1.0", default-features = false }
```

### With CLI Tool (Automatically includes CRC validation)

To include the command-line tool, enable the `cli` feature:

```toml
[dependencies]
scte35-parsing = { version = "0.1.0", features = ["cli"] }
```

Or install the CLI tool directly:

```bash
cargo install scte35-parsing --features cli
```

**Note**: The CLI feature automatically enables CRC validation to provide complete message diagnostics.

## Usage

### Library Usage

```rust
use scte35_parsing::{parse_splice_info_section, SpliceCommand, SpliceDescriptor};
use std::time::Duration;

// Your SCTE-35 message as bytes (example message)
let scte35_bytes = vec![
    0xFC, 0x30, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xF0, 0x05, 0x06, 0xFE, 
    0x42, 0x3A, 0x35, 0xBD, 0x00, 0x00, 0xBB, 0x0C, 0x73, 0xF4
];

match parse_splice_info_section(&scte35_bytes) {
Ok(section) => {
    println!("Table ID: {}", section.table_id);
    println!("Command Type: {}", section.splice_command_type);
    
    match section.splice_command {
        SpliceCommand::SpliceInsert(insert) => {
            println!("Splice Event ID: 0x{:08x}", insert.splice_event_id);
            
            // Convert break duration to std::time::Duration
            if let Some(break_duration) = &insert.break_duration {
                let duration: Duration = break_duration.into();
                println!("Break Duration: {:?}", duration);
                println!("Break Duration: {:.3} seconds", duration.as_secs_f64());
            }
            
            // Convert splice time to Duration
            if let Some(duration) = insert.splice_time.as_ref()
                .and_then(|st| st.to_duration()) {
                println!("Splice Time: {:?}", duration);
            }
        }
        SpliceCommand::TimeSignal(signal) => {
            if let Some(duration) = signal.splice_time.to_duration() {
                println!("Time Signal: {:?}", duration);
            }
        }
        _ => println!("Other command type"),
    }
    
    // Parse segmentation descriptors with UPID information
    for descriptor in &section.splice_descriptors {
        if let SpliceDescriptor::Segmentation(seg_desc) = descriptor {
            println!("Segmentation Event ID: 0x{:08x}", seg_desc.segmentation_event_id);
            println!("UPID Type: {}", seg_desc.upid_type_description());
            println!("Segmentation Type: {}", seg_desc.segmentation_type_description());
            
            if let Some(upid_str) = seg_desc.upid_as_string() {
                println!("UPID: {}", upid_str);
            }
        }
    }
}
Err(e) => eprintln!("Error parsing SCTE-35: {}", e),
}
```

### CRC Validation

By default, the library validates CRC-32 checksums in SCTE-35 messages to ensure data integrity:

```rust
use scte35_parsing::{parse_splice_info_section, validate_scte35_crc, CrcValidatable};

// Example SCTE-35 message bytes
let scte35_bytes = vec![
    0xFC, 0x30, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xF0, 0x05, 0x06, 0xFE, 
    0x42, 0x3A, 0x35, 0xBD, 0x00, 0x00, 0xBB, 0x0C, 0x73, 0xF4
];

// Parse with automatic CRC validation (default behavior)
match parse_splice_info_section(&scte35_bytes) {
Ok(section) => {
    println!("Valid SCTE-35 message parsed successfully");
    println!("CRC-32: 0x{:08X}", section.get_crc());
}
Err(e) => {
    if e.to_string().contains("CRC validation failed") {
        eprintln!("Message corrupted or tampered: {}", e);
    } else {
        eprintln!("Parse error: {}", e);
    }
}
}

// Validate CRC independently
match validate_scte35_crc(&scte35_bytes) {
    Ok(true) => println!("CRC validation passed"),
    Ok(false) => println!("CRC validation failed or not available"),
    Err(e) => eprintln!("Validation error: {}", e),
}

// Validate using the parsed section
if let Ok(section) = parse_splice_info_section(&scte35_bytes) {
    match section.validate_crc(&scte35_bytes) {
        Ok(true) => println!("Message integrity verified"),
        Ok(false) => println!("Message integrity check failed"),
        Err(e) => eprintln!("Validation error: {}", e),
    }
}
```

### Duration Conversion

SCTE-35 time values are represented as 90kHz clock ticks. This library provides convenient conversion to Rust's `std::time::Duration`:

```rust
use scte35_parsing::BreakDuration;
use std::time::Duration;

// Create a break duration of 30 seconds (30 * 90000 ticks)
let break_duration = BreakDuration {
    auto_return: 1,
    reserved: 0,
    duration: 2_700_000,
};

// Convert using Into trait  
let duration: Duration = (&break_duration).into();
assert_eq!(duration.as_secs(), 30);

// Or use the method directly
let duration2 = break_duration.to_duration();
assert_eq!(duration2.as_secs(), 30);
```

### Serde Support (JSON Serialization)

The library includes built-in serde support for serializing/deserializing SCTE-35 messages:

```rust
use scte35_parsing::parse_splice_info_section;
use serde_json;

// Parse SCTE-35 message
let scte35_bytes = vec![
    0xFC, 0x30, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xF0, 0x05, 0x06, 0xFE, 
    0x42, 0x3A, 0x35, 0xBD, 0x00, 0x00, 0xBB, 0x0C, 0x73, 0xF4
];

if let Ok(section) = parse_splice_info_section(&scte35_bytes) {
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&section).unwrap();
    println!("{}", json);
    
    // Deserialize from JSON
    let deserialized: scte35_parsing::SpliceInfoSection = 
        serde_json::from_str(&json).unwrap();
    assert_eq!(section, deserialized);
}
```

The serde implementation includes:

- **Binary data as base64**: All raw bytes (private commands, UPID data, alignment bits) are encoded as base64 strings
- **Human-readable enums**: Segmentation types and UPID types include both numeric values and descriptions
- **Time duration info**: PTS times and durations include both raw ticks and human-readable formats
- **Computed fields**: Segmentation descriptors include parsed UPID strings when available

Example JSON output:
```json
{
  "table_id": 252,
  "section_syntax_indicator": 0,
  "private_indicator": 0,
  "section_length": 22,
  "protocol_version": 0,
  "encrypted_packet": 0,
  "encryption_algorithm": 0,
  "pts_adjustment": 0,
  "cw_index": 255,
  "tier": 4095,
  "splice_command_length": 5,
  "splice_command_type": 6,
  "splice_command": {
    "type": "TimeSignal",
    "splice_time": {
      "time_specified_flag": 1,
      "pts_time": 900000,
      "duration_info": {
        "ticks": 900000,
        "seconds": 10.0,
        "human_readable": "10.0s"
      }
    }
  },
  "descriptor_loop_length": 0,
  "splice_descriptors": [],
  "alignment_stuffing_bits": "",
  "e_crc_32": null,
  "crc_32": 0
}
```

### Segmentation Types

The library provides human-readable segmentation types that correspond to the numeric IDs in SCTE-35 messages:

```rust
use scte35_parsing::{SegmentationType, parse_splice_info_section, SpliceDescriptor};

// Work with segmentation types directly
let seg_type = SegmentationType::ProviderAdvertisementStart;
println!("Type: {} (ID: 0x{:02X})", seg_type, seg_type.id());

// Convert from numeric ID (useful when parsing)
let seg_type = SegmentationType::from_id(0x30);
assert_eq!(seg_type, SegmentationType::ProviderAdvertisementStart);
assert_eq!(seg_type.to_string(), "Provider Advertisement Start");

// Example: Parse a message and check segmentation descriptors
let scte35_bytes = vec![
    0xFC, 0x30, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xF0, 0x05, 0x06, 0xFE, 
    0x42, 0x3A, 0x35, 0xBD, 0x00, 0x00, 0xBB, 0x0C, 0x73, 0xF4
];

if let Ok(section) = parse_splice_info_section(&scte35_bytes) {
    // Segmentation descriptors automatically populate both fields
    // The numeric ID and human-readable type are always consistent
    for descriptor in &section.splice_descriptors {
        if let SpliceDescriptor::Segmentation(seg_desc) = descriptor {
            println!("Segmentation Type ID: 0x{:02X}", seg_desc.segmentation_type_id);
            println!("Segmentation Type: {:?}", seg_desc.segmentation_type);
            println!("Description: {}", seg_desc.segmentation_type_description());
        }
    }
}
```

#### Supported Segmentation Types

The library supports all standard SCTE-35 segmentation types including:

**Program Boundaries:**
- Program Start/End
- Program Early Termination
- Program Breakaway/Resumption
- Program Runover (Planned/Unplanned)
- Program Overlap Start
- Program Blackout Override
- Program Join

**Content Segments:**
- Chapter Start/End
- Break Start/End
- Content Identification

**Advertisement Opportunities:**
- Provider/Distributor Advertisement Start/End
- Provider/Distributor Placement Opportunity Start/End
- Provider/Distributor Overlay Placement Opportunity Start/End
- Provider/Distributor Promo Start/End
- Provider/Distributor Ad Block Start/End

**Special Events:**
- Unscheduled Event Start/End
- Alternate Content Opportunity Start/End
- Network Start/End

### Builder Pattern API (Creating SCTE-35 Messages)

The library includes a comprehensive builder pattern API for creating SCTE-35 messages from scratch with type safety and validation:

```rust
# use scte35_parsing::builders::*;
# use scte35_parsing::types::SegmentationType;
# use std::time::Duration;
# fn main() -> BuilderResult<()> {
// Example 1: Creating a 30-second ad break starting at 20 seconds
let splice_insert = SpliceInsertBuilder::new(12345)
    .at_pts(Duration::from_secs(20))?
    .duration(Duration::from_secs(30))
    .unique_program_id(0x1234)
    .avail(1, 4)  // First of 4 avails
    .build()?;

let section = SpliceInfoSectionBuilder::new()
    .pts_adjustment(0)
    .splice_insert(splice_insert)
    .build()?;

println!("Created SCTE-35 message with {} byte payload", section.section_length);
# Ok(())
# }
```

#### Creating Time Signals with Segmentation Descriptors

```rust
# use scte35_parsing::builders::*;
# use scte35_parsing::types::SegmentationType;
# use std::time::Duration;
# fn example() -> BuilderResult<()> {
// Example 2: Program start boundary with UPID
let segmentation = SegmentationDescriptorBuilder::new(
        5678, 
        SegmentationType::ProgramStart
    )
    .upid(Upid::AdId("ABC123456789".to_string()))?
    .duration(Duration::from_secs(1800))?  // 30-minute program
    .build()?;

let section = SpliceInfoSectionBuilder::new()
    .time_signal(TimeSignalBuilder::new().immediate().build()?)
    .add_segmentation_descriptor(segmentation)
    .build()?;
# Ok(())
# }
```

#### Component-Level Splice Operations

```rust
# use scte35_parsing::builders::*;
# use std::time::Duration;
# fn example() -> BuilderResult<()> {
// Example 3: Component-level splice for specific audio/video streams
let splice_insert = SpliceInsertBuilder::new(3333)
    .component_splice(vec![
        (0x01, Some(Duration::from_secs(10))),  // Video component
        (0x02, Some(Duration::from_secs(10))),  // Audio component 1
        (0x03, Some(Duration::from_secs(10))),  // Audio component 2
    ])?
    .duration(Duration::from_secs(15))
    .build()?;
# Ok(())
# }
```

#### Advanced Segmentation with Delivery Restrictions

```rust
# use scte35_parsing::builders::*;
# use scte35_parsing::types::SegmentationType;
# fn example() -> BuilderResult<()> {
// Example 4: Complex segmentation with delivery restrictions
let restrictions = DeliveryRestrictions {
    web_delivery_allowed: false,
    no_regional_blackout: false,
    archive_allowed: true,
    device_restrictions: DeviceRestrictions::RestrictGroup1,
};

let uuid_bytes = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];

let segmentation = SegmentationDescriptorBuilder::new(
        7777,
        SegmentationType::DistributorAdvertisementStart
    )
    .delivery_restrictions(restrictions)
    .upid(Upid::Uuid(uuid_bytes))?
    .segment(2, 6)  // 2nd of 6 segments
    .build()?;
# Ok(())
# }
```

#### Comprehensive UPID Support

The builder API supports all SCTE-35 UPID types with validation:

```rust
# use scte35_parsing::builders::*;
# fn example() -> BuilderResult<()> {
# let mut builder = SegmentationDescriptorBuilder::new(1, scte35_parsing::types::SegmentationType::ProgramStart);
// Ad ID (12 ASCII characters)
builder = builder.upid(Upid::AdId("ABC123456789".to_string()))?;

// UUID (16 bytes)
let uuid_bytes = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
builder = builder.upid(Upid::Uuid(uuid_bytes))?;

// URI (variable length)
builder = builder.upid(Upid::Uri("https://example.com/content/123".to_string()))?;

// ISAN (12 bytes)
let isan_bytes = [0x00, 0x00, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23];
builder = builder.upid(Upid::Isan(isan_bytes))?;

// And many more: UMID, EIDR, TID, AiringID, etc.
# Ok(())
# }
```

#### Error Handling and Validation

The builder API provides comprehensive validation with clear error messages:

```rust
# use scte35_parsing::builders::*;
# use scte35_parsing::types::SegmentationType;
# use std::time::Duration;
# fn example() -> BuilderResult<()> {
// Invalid UPID length
let result = SegmentationDescriptorBuilder::new(1234, SegmentationType::ProgramStart)
    .upid(Upid::AdId("TOO_SHORT".to_string()));

match result {
    Err(BuilderError::InvalidUpidLength { expected, actual }) => {
        println!("UPID validation failed: expected {} chars, got {}", expected, actual);
    }
    _ => {}
}

// Duration too large for 33-bit PTS
let result = SpliceInsertBuilder::new(1234)
    .at_pts(Duration::from_secs(u64::MAX / 90_000 + 1))?
    .build();

match result {
    Err(BuilderError::DurationTooLarge { field, duration }) => {
        println!("Duration {} is too large for field {}", duration.as_secs(), field);
    }
    _ => {}
}
# Ok(())
# }
```

#### Builder Features

- **Type Safety**: Compile-time prevention of invalid message states
- **Validation**: Runtime validation with descriptive error messages  
- **Ergonomic API**: Fluent interface with sensible defaults
- **Spec Compliance**: Automatic handling of reserved fields and constraints
- **Complete Coverage**: Builders for all major SCTE-35 structures

See the [builder_demo example](examples/builder_demo.rs) for more comprehensive usage examples.

### CLI Usage

When built with the `cli` feature, you can parse base64-encoded SCTE-35 messages with multiple output formats:

```bash
# Text output (default)
cargo run --features cli -- "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo="

# JSON output
cargo run --features cli -- -o json "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo="

# Or with long flag
cargo run --features cli -- --output json "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo="

# Get help
cargo run --features cli -- --help
```

Example output with splice insert command and break duration:
```text
Successfully parsed SpliceInfoSection:
  Table ID: 252
  Section Length: 47
  Protocol Version: 0
  Splice Command Type: 5
  Splice Command Length: 20
  Splice Command: SpliceInsert
    Splice Event ID: 0x4800008f
    Splice Event Cancel: 0
    Out of Network: 1
    Program Splice Flag: 1
    Duration Flag: 1
    Splice Immediate Flag: 0
    Splice Time PTS: 0x07369c02e
    Splice Time: 21514.559089 seconds
    Break Duration:
      Auto Return: 1
      Duration: 0x00052ccf5 (60.293567 seconds)
    Unique Program ID: 0
    Avail Num: 0
    Avails Expected: 0
  Descriptor Loop Length: 10
  Number of Descriptors: 1
    Unknown Descriptor:
      Tag: 0x00
      Length: 8
      Content: "CUEI  5"
  CRC-32: 0x62DBA30A ✓ (Valid)
```

#### CLI Output Formats

The CLI supports two output formats:

- **Text format** (default): Human-readable format with detailed field descriptions
- **JSON format**: Structured JSON output for programmatic use

JSON output includes the complete parsed structure with CRC validation results:

```bash
cargo run --features cli -- -o json "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo="
```

```json
{
  "status": "success",
  "data": {
    "table_id": 252,
    "section_length": 47,
    "protocol_version": 0,
    "splice_command_type": 5,
    "splice_command_length": 20,
    "splice_command": {
      "type": "SpliceInsert",
      "splice_event_id": 1207959695,
      "splice_event_cancel_indicator": 0,
      "out_of_network_indicator": 1,
      "program_splice_flag": 1,
      "duration_flag": 1,
      "splice_immediate_flag": 0,
      "splice_time": {
        "time_specified_flag": 1,
        "pts_time": 1936310318,
        "duration_info": {
          "ticks": 1936310318,
          "seconds": 21514.559088888887,
          "human_readable": "5h 58m 34.6s"
        }
      },
      "break_duration": {
        "auto_return": 1,
        "duration": 5426421,
        "duration_info": {
          "ticks": 5426421,
          "seconds": 60.29356666666666,
          "human_readable": "1m 0.3s"
        }
      },
      "unique_program_id": 0,
      "avail_num": 0,
      "avails_expected": 0
    },
    "descriptor_loop_length": 10,
    "splice_descriptors": [
      {
        "descriptor_type": "Unknown",
        "tag": 0,
        "length": 8,
        "data": "Q1VFSQAAATU="
      }
    ],
    "crc_32": 1658561290
  },
  "crc_validation": {
    "valid": true,
    "error": null
  }
}
```

## Supported SCTE-35 Commands

- **SpliceNull** - Null command
- **SpliceSchedule** - Scheduled splice events
- **SpliceInsert** - Immediate or scheduled ad insertion points
- **TimeSignal** - Time synchronization signals
- **BandwidthReservation** - Bandwidth allocation commands
- **PrivateCommand** - Private/custom commands

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

### Benefits of CRC Validation

1. **Data Integrity**: Ensures SCTE-35 messages haven't been corrupted during transmission
2. **Security**: Helps detect tampered messages
3. **Debugging**: Identifies parsing issues vs. data corruption
4. **Standards Compliance**: Follows SCTE-35 specification requirements
5. **Flexibility**: Optional feature allows users to choose performance vs. validation trade-offs

## API Documentation

Full API documentation is available at [docs.rs](https://docs.rs/scte35-parsing) or can be generated locally:

```bash
cargo doc --no-deps --open
```

### Main Functions

#### `parse_splice_info_section(buffer: &[u8]) -> Result<SpliceInfoSection, io::Error>`

Parses a complete SCTE-35 splice information section from a byte buffer. Automatically validates CRC-32 when the `crc-validation` feature is enabled.

#### `validate_scte35_crc(buffer: &[u8]) -> Result<bool, io::Error>`

Validates the CRC-32 checksum of an SCTE-35 message independently. Returns `Ok(true)` if valid, `Ok(false)` if invalid or CRC validation is disabled.

### Data Structures

#### `SpliceInfoSection`
The top-level structure containing all SCTE-35 message fields:
- `table_id`: Table identifier (should be 0xFC for SCTE-35)
- `section_length`: Length of the section
- `protocol_version`: SCTE-35 protocol version
- `splice_command_type`: Type of splice command
- `splice_command`: The actual command data (enum)
- `descriptor_loop_length`: Length of descriptors
- `splice_descriptors`: List of splice descriptors
- `crc_32`: CRC32 checksum

#### `SpliceCommand`
An enum representing different SCTE-35 command types:
- `SpliceNull`
- `SpliceSchedule(SpliceSchedule)`
- `SpliceInsert(SpliceInsert)`
- `TimeSignal(TimeSignal)`
- `BandwidthReservation(BandwidthReservation)`
- `PrivateCommand(PrivateCommand)`
- `Unknown`

## Building from Source

### Build Library Only
```bash
git clone https://github.com/yourusername/scte35-parsing
cd scte35-parsing
cargo build --release
```

### Build with CLI Tool
```bash
cargo build --release --features cli
```

### Run Tests
```bash
cargo test
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## References

- [SCTE-35 2023r1 Specification](https://www.scte.org/standards/)
- [Digital Program Insertion Cueing Message](https://en.wikipedia.org/wiki/SCTE-35)