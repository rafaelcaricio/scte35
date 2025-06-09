# Builder Pattern Module

This module provides a comprehensive builder pattern API for creating SCTE-35 messages from scratch with type safety and validation.

## Module Overview

The builder pattern implementation allows users to construct valid SCTE-35 messages programmatically rather than just parsing existing ones. This is essential for applications that need to generate SCTE-35 cue messages for ad insertion, content segmentation, or other broadcast automation tasks.

## Architecture

### Core Components

1. **Error Handling** (`error.rs`)
   - `BuilderError` enum with descriptive error variants
   - `BuilderResult<T>` type alias for consistent error handling
   - `DurationExt` trait for PTS time conversion utilities

2. **Command Builders** (`commands.rs`)
   - `SpliceInsertBuilder` - Creates splice insert commands for ad breaks
   - `TimeSignalBuilder` - Creates time signal commands for timed events

3. **Descriptor Builders** (`descriptors.rs`)
   - `SegmentationDescriptorBuilder` - Creates segmentation descriptors with UPID support
   - Full support for all 18 UPID types from SCTE-35 specification

4. **Time Utilities** (`time.rs`)
   - `SpliceTimeBuilder` - Handles PTS time specifications
   - `BreakDurationBuilder` - Manages break duration encoding
   - `DateTimeBuilder` - Creates UTC timestamp structures

5. **Section Builder** (`splice_info_section.rs`)
   - `SpliceInfoSectionBuilder` - Top-level builder for complete SCTE-35 messages
   - Handles command and descriptor aggregation

6. **Extensions** (`extensions.rs`)
   - `SpliceCommandExt` trait - Provides encoding utilities for commands
   - Helper traits for internal builder operations

## Design Principles

### Type Safety
- Builders consume `self` to prevent reuse after `build()`
- Compile-time prevention of invalid states
- Strong typing for all SCTE-35 structures

### Validation
- Runtime validation with descriptive error messages
- Automatic bit-width masking (e.g., 33-bit PTS, 12-bit tier)
- UPID length validation based on type
- Component count limits enforcement

### Ergonomics
- Fluent interface with method chaining
- Sensible defaults for optional fields
- Convenient duration conversions from `std::time::Duration`
- Clear error messages for debugging

## Common Patterns

### Creating a Splice Insert
```rust
let splice_insert = SpliceInsertBuilder::new(event_id)
    .at_pts(Duration::from_secs(20))?
    .duration(Duration::from_secs(30))
    .out_of_network(true)
    .build()?;
```

### Creating a Segmentation Descriptor
```rust
let descriptor = SegmentationDescriptorBuilder::new(event_id, SegmentationType::ProgramStart)
    .upid(Upid::AdId("ABC123456789".to_string()))?
    .duration(Duration::from_secs(1800))?
    .build()?;
```

### Building Complete Messages
```rust
let section = SpliceInfoSectionBuilder::new()
    .splice_insert(splice_insert)
    .add_segmentation_descriptor(descriptor)
    .build()?;
```

## UPID Support

The builder supports all 18 UPID types defined in SCTE-35:
- `None` - No UPID
- `UserDefinedDeprecated` - Deprecated user-defined format
- `Isci` - ISCI commercial code (8 characters)
- `AdId` - Ad-ID (12 characters)
- `Umid` - SMPTE UMID (32 bytes)
- `IsanDeprecated` - Deprecated ISAN (12 bytes)
- `Isan` - V-ISAN (12 bytes)
- `Tid` - Tribune ID (12 characters)
- `AiringId` - Airing ID (64-bit)
- `Adi` - ADI (variable length)
- `Eidr` - EIDR (12 bytes)
- `AtscContentIdentifier` - ATSC content identifier
- `Mpu` - Managed Private UPID
- `Mid` - Multiple UPID
- `AdsInformation` - ADS information
- `Uri` - URI (variable length)
- `Uuid` - UUID (16 bytes)
- `Scr` - Source Content Reference
- `Reserved` - Reserved types

## Testing

The module includes comprehensive tests in `tests.rs`:
- Unit tests for each builder
- Validation error tests
- Edge case handling (overflow, limits)
- Integration tests with complete messages

## Future Enhancements

Potential areas for expansion:
- Additional descriptor types (DTMF, Avail, etc.)
- Batch message creation utilities
- Template-based message generation
- Serialization helpers for specific use cases