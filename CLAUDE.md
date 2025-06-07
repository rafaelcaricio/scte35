# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust library for parsing SCTE-35 (Society of Cable Telecommunications Engineers) messages. SCTE-35 is a standard for inserting cue messages into video streams, commonly used for ad insertion points in broadcast television.

## Common Development Commands

### Build Library (No Dependencies)
```bash
cargo build
```

### Build with CLI Tool
```bash
cargo build --features cli
```

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

- The library has zero dependencies by default
- The `cli` feature adds `base64` as a dependency for the CLI tool
- `base64` is also included as a dev dependency for testing with encoded SCTE-35 samples
- When adding new command types or fields, ensure proper bit alignment and offset tracking
- Test coverage includes parsing real SCTE-35 messages encoded in base64