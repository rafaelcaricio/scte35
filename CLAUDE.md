# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust library for parsing SCTE-35 (Society of Cable Telecommunications Engineers) messages. SCTE-35 is a standard for inserting cue messages into video streams, commonly used for ad insertion points in broadcast television.

## Common Development Commands

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

### Run CLI Tool
```bash
cargo run --features cli -- <base64-encoded-scte35-payload>
```

### Run Tests
```bash
cargo test
```

### Fix Compilation Warnings
```bash
cargo fix --lib -p scte35-parsing
```

### Check Code Without Building
```bash
cargo check
```

### Generate Documentation
```bash
cargo doc --no-deps --open
```

### Run Tests for a Specific Test
```bash
cargo test test_parse_splice_info_section
```

## Architecture

The codebase implements a bit-level parser for SCTE-35 binary messages:

- **BitReader**: A custom bit-level reader that can extract values from byte buffers at arbitrary bit offsets
- **Data Structures**: Rust structs representing SCTE-35 message components (SpliceInfoSection, SpliceCommand variants, descriptors)
- **Parser Functions**: Functions that use BitReader to parse binary data into the structured format

Key components:
- `parse_splice_info_section()`: Main entry point for parsing SCTE-35 messages
- Support for multiple splice command types: SpliceNull, SpliceSchedule, SpliceInsert, TimeSignal, BandwidthReservation, PrivateCommand
- Handles bit-level field extraction following SCTE-35 specification

## Development Notes

- The library includes CRC validation by default via the `crc-validation` feature
- The `crc-validation` feature adds the `crc` crate as a dependency for MPEG-2 CRC validation
- The `cli` feature adds `base64` as a dependency and automatically enables `crc-validation`
- `base64` is also included as a dev dependency for testing with encoded SCTE-35 samples
- When adding new command types or fields, ensure proper bit alignment and offset tracking
- Test coverage includes parsing real SCTE-35 messages encoded in base64
- CRC validation ensures data integrity and helps detect corrupted messages

## Development Guidelines

- Always add test cases to cover new functionality
- Follow comprehensive documentation standards (see Documentation Guidelines below)

## Documentation Guidelines

This project follows strict documentation standards. All changes must include proper documentation:

### Required Documentation

1. **All public items must be documented** - The codebase uses `#![warn(missing_docs)]` to enforce this
2. **Module-level documentation** - Comprehensive overview with examples in `lib.rs`
3. **Function documentation** - Purpose, parameters, return values, and examples
4. **Struct documentation** - Purpose and usage context
5. **Field documentation** - Meaning and valid values for each public field
6. **Enum variant documentation** - Purpose of each variant

### Documentation Standards

1. **Use `///` for public items** and `//` for internal comments
2. **Start with a brief summary** - One line describing what the item does
3. **Include examples** when helpful, especially for public functions
4. **Explain SCTE-35 context** - What this represents in the specification
5. **Document bit fields and flags** - Explain what 0/1 values mean
6. **Include units** - Specify when values are in 90kHz ticks, bytes, etc.
7. **Cross-reference** - Link to related structs/functions when appropriate

### Example Documentation Format

```rust
/// Represents a splice insert command for ad insertion points.
///
/// This is the most commonly used command for indicating where
/// advertisements should be inserted into the video stream.
#[derive(Debug)]
pub struct SpliceInsert {
    /// Unique identifier for the splice event
    pub splice_event_id: u32,
    /// Flag indicating if this event should be cancelled (0 = proceed, 1 = cancel)
    pub splice_event_cancel_indicator: u8,
    /// Duration of the break/ad insertion in 90kHz ticks
    pub break_duration: Option<BreakDuration>,
}
```

### Documentation Commands

```bash
# Generate and open documentation
cargo doc --no-deps --open

# Check for missing documentation
cargo check  # Will show warnings for missing docs

# Test documentation examples
cargo test  # Includes doctests
```

### Before Committing

1. Run `cargo check` and ensure no missing documentation warnings
2. Run `cargo doc --no-deps` to verify documentation builds correctly
3. Run `cargo test` to ensure doc examples work
4. Review generated documentation for clarity and completeness