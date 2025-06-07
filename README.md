# SCTE-35 Parsing Library

A zero-dependency Rust library for parsing SCTE-35 (Society of Cable Telecommunications Engineers) messages. SCTE-35 is a standard for inserting cue messages into video streams, commonly used for ad insertion points in broadcast television.

## Features

- **Zero dependencies** - The core library has no runtime dependencies
- **Full SCTE-35 parsing** - Supports all major SCTE-35 command types
- **Bit-level precision** - Accurate parsing of bit-packed SCTE-35 messages
- **Optional CLI tool** - Command-line interface for parsing base64-encoded messages
- **Type-safe** - Strongly typed representations of all SCTE-35 structures

## Installation

### As a Library (No Dependencies)

Add this to your `Cargo.toml`:

```toml
[dependencies]
scte35-parsing = "0.1.0"
```

### With CLI Tool

To include the command-line tool, enable the `cli` feature:

```toml
[dependencies]
scte35-parsing = { version = "0.1.0", features = ["cli"] }
```

Or install the CLI tool directly:

```bash
cargo install scte35-parsing --features cli
```

## Usage

### Library Usage

```rust
use scte35_parsing::{parse_splice_info_section, SpliceCommand};
use std::time::Duration;

fn main() {
    // Your SCTE-35 message as bytes
    let scte35_bytes = vec![/* your SCTE-35 bytes */];
    
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
        }
        Err(e) => eprintln!("Error parsing SCTE-35: {}", e),
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
let duration: Duration = break_duration.into();
assert_eq!(duration.as_secs(), 30);

// Or use the method directly
let duration = break_duration.to_duration();

// Also works with references
let duration: Duration = (&break_duration).into();
```

### CLI Usage

When built with the `cli` feature, you can parse base64-encoded SCTE-35 messages:

```bash
# Run with cargo
cargo run --features cli -- "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo="

# Or if installed
scte35-parsing "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo="
```

Example output:
```
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
    Splice Time PTS: 0x07369c02e (21514.559089 seconds)
    Break Duration:
      Auto Return: 1
      Duration: 0x00052ccf5 (60.293567 seconds)
    Unique Program ID: 0
    Avail Num: 0
    Avails Expected: 0
  Descriptor Loop Length: 10
  Number of Descriptors: 1
    Descriptor Tag: 0
    Descriptor Length: 8
  CRC-32: 1658561290
```

## Supported SCTE-35 Commands

- **SpliceNull** - Null command
- **SpliceSchedule** - Scheduled splice events
- **SpliceInsert** - Immediate or scheduled ad insertion points
- **TimeSignal** - Time synchronization signals
- **BandwidthReservation** - Bandwidth allocation commands
- **PrivateCommand** - Private/custom commands

## API Documentation

### Main Functions

#### `parse_splice_info_section(buffer: &[u8]) -> Result<SpliceInfoSection, io::Error>`

Parses a complete SCTE-35 splice information section from a byte buffer.

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