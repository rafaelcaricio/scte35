# SCTE-35 Parsing Library

A Rust library for parsing SCTE-35 (Society of Cable Telecommunications Engineers) messages with built-in CRC validation. SCTE-35 is a standard for inserting cue messages into video streams, commonly used for ad insertion points in broadcast television.

## Features

- **CRC validation** - Built-in CRC-32 validation using MPEG-2 algorithm (enabled by default)
- **Human-readable UPID parsing** - Full support for 18 standard UPID types with intelligent formatting
- **Human-readable segmentation types** - Complete set of 48 standard segmentation types with descriptive names
- **Segmentation descriptor parsing** - Complete parsing of segmentation descriptors including UPID data
- **Minimal dependencies** - Only the `crc` crate for validation (optional)
- **Full SCTE-35 parsing** - Supports all major SCTE-35 command types
- **Bit-level precision** - Accurate parsing of bit-packed SCTE-35 messages
- **Optional CLI tool** - Command-line interface for parsing base64-encoded messages with UPID display
- **Type-safe** - Strongly typed representations of all SCTE-35 structures
- **Data integrity** - Detects corrupted or tampered SCTE-35 messages

## Installation

### With CRC Validation (Default)

Add this to your `Cargo.toml`:

```toml
[dependencies]
scte35-parsing = "0.1.0"
```

### Without CRC Validation (Library only)

If you need a zero-dependency library without CRC validation:

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

fn main() {
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

### Segmentation Types

The library provides human-readable segmentation types that correspond to the numeric IDs in SCTE-35 messages:

```rust
use scte35_parsing::{SegmentationType, parse_splice_info_section, SpliceDescriptor};

// Work with segmentation types directly
let seg_type = SegmentationType::ProviderAdvertisementStart;
println!("Type: {} (ID: 0x{:02X})", seg_type.description(), seg_type.id());

// Convert from numeric ID (useful when parsing)
let seg_type = SegmentationType::from_id(0x30);
assert_eq!(seg_type, SegmentationType::ProviderAdvertisementStart);
assert_eq!(seg_type.description(), "Provider Advertisement Start");

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

### CLI Usage

When built with the `cli` feature, you can parse base64-encoded SCTE-35 messages:

```bash
# Run with cargo
cargo run --features cli -- "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo="

# Or if installed
scte35-parsing "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo="
```

Example output with segmentation descriptor and UPID information:
```text
Successfully parsed SpliceInfoSection:
  Table ID: 252
  Section Length: 67
  Protocol Version: 0
  Splice Command Type: 6
  Splice Command Length: 5
  Splice Command: TimeSignal
    PTS Time: 888889
  Descriptor Loop Length: 45
  Number of Descriptors: 1
    Segmentation Descriptor:
      Event ID: 0x00000003
      Cancel Indicator: false
      Program Segmentation: true
      Duration Flag: false
      UPID Type: UMID (Unique Material Identifier) (0x04)
      UPID Length: 28 bytes
      UPID: MDYwYTJiMzQuMDEwMTAxMDUuMDEwMTBkMjAuMQ==
      Segmentation Type ID: 0x10
      Segmentation Type: Program Start
      Segment Number: 1
      Segments Expected: 1
  CRC-32: 0x44A237BE ✓ (Valid)
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