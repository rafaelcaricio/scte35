# Builder Pattern API for SCTE-35 Message Creation

## Overview

This plan describes the implementation of a Builder Pattern API for creating SCTE-35 messages from scratch. The API is designed for broadcast systems that need to generate valid SCTE-35 splice commands for ad insertion and content segmentation.

## Design Principles

1. **Type Safety**: Leverage Rust's type system to prevent invalid states at compile time
2. **Spec Compliance**: Hide reserved/fixed fields and enforce SCTE-35 specification constraints
3. **Ergonomic API**: Fluent interface with sensible defaults and clear method names
4. **Validation**: Validate inputs early and provide clear error messages
5. **Completeness**: Provide builders for all public structs

## Builder Implementation Strategy

### Module Structure

```rust
// src/builders.rs
pub mod splice_info_section;
pub mod commands;
pub mod descriptors;
pub mod time;

// Re-export builders at module level
pub use splice_info_section::SpliceInfoSectionBuilder;
pub use commands::*;
pub use descriptors::*;
pub use time::*;
```

### Error Handling

```rust
use std::error::Error;
use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum BuilderError {
    MissingRequiredField(&'static str),
    InvalidValue { field: &'static str, reason: String },
    DurationTooLarge { field: &'static str, duration: Duration },
    InvalidUpidLength { expected: usize, actual: usize },
    InvalidComponentCount { max: usize, actual: usize },
}

impl fmt::Display for BuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuilderError::MissingRequiredField(field) => 
                write!(f, "Required field '{}' is missing", field),
            BuilderError::InvalidValue { field, reason } => 
                write!(f, "Invalid value for field '{}': {}", field, reason),
            BuilderError::DurationTooLarge { field, duration } => 
                write!(f, "Duration for field '{}' is too large: {:?} exceeds 33-bit PTS limit", field, duration),
            BuilderError::InvalidUpidLength { expected, actual } => 
                write!(f, "Invalid UPID length: expected {} bytes, got {}", expected, actual),
            BuilderError::InvalidComponentCount { max, actual } => 
                write!(f, "Too many components: maximum {}, got {}", max, actual),
        }
    }
}

impl Error for BuilderError {}

pub type BuilderResult<T> = Result<T, BuilderError>;

/// Helper trait to convert Duration to 90kHz PTS ticks
trait DurationExt {
    fn to_pts_ticks(&self) -> u64;
}

impl DurationExt for Duration {
    fn to_pts_ticks(&self) -> u64 {
        self.as_secs() * 90_000 + (self.subsec_nanos() as u64 * 90_000 / 1_000_000_000)
    }
}
```

## Struct Builders

### 1. SpliceInfoSectionBuilder

The top-level builder for creating complete SCTE-35 messages.

```rust
pub struct SpliceInfoSectionBuilder {
    pts_adjustment: u64,
    tier: u16,
    splice_command: Option<SpliceCommand>,
    descriptors: Vec<SpliceDescriptor>,
}

impl SpliceInfoSectionBuilder {
    pub fn new() -> Self {
        Self {
            pts_adjustment: 0,
            tier: 0xFFF, // Default "all tiers"
            splice_command: None,
            descriptors: Vec::new(),
        }
    }

    pub fn pts_adjustment(mut self, pts_adjustment: u64) -> Self {
        self.pts_adjustment = pts_adjustment & 0x1_FFFF_FFFF; // 33-bit value
        self
    }

    pub fn tier(mut self, tier: u16) -> Self {
        self.tier = tier & 0xFFF; // 12-bit value
        self
    }

    pub fn splice_command(mut self, command: SpliceCommand) -> Self {
        self.splice_command = Some(command);
        self
    }

    pub fn splice_null(mut self) -> Self {
        self.splice_command = Some(SpliceCommand::SpliceNull);
        self
    }

    pub fn splice_insert(mut self, insert: SpliceInsert) -> Self {
        self.splice_command = Some(SpliceCommand::SpliceInsert(insert));
        self
    }

    pub fn time_signal(mut self, time_signal: TimeSignal) -> Self {
        self.splice_command = Some(SpliceCommand::TimeSignal(time_signal));
        self
    }

    pub fn add_descriptor(mut self, descriptor: SpliceDescriptor) -> Self {
        self.descriptors.push(descriptor);
        self
    }

    pub fn add_segmentation_descriptor(mut self, descriptor: SegmentationDescriptor) -> Self {
        self.descriptors.push(SpliceDescriptor::Segmentation(descriptor));
        self
    }

    pub fn build(self) -> BuilderResult<SpliceInfoSection> {
        let splice_command = self.splice_command
            .ok_or(BuilderError::MissingRequiredField("splice_command"))?;

        // Calculate section_length and other derived fields
        let splice_command_length = splice_command.encoded_length();
        let descriptor_loop_length = self.descriptors.iter()
            .map(|d| 2 + d.length() as u16)
            .sum::<u16>();
        let section_length = 11 + splice_command_length + 2 + descriptor_loop_length + 4;

        Ok(SpliceInfoSection {
            table_id: 0xFC,  // Fixed per spec
            section_syntax_indicator: 0,  // Fixed per spec
            private_indicator: 0,  // Fixed per spec
            sap_type: 0x3,  // Fixed per spec (undefined)
            section_length,
            protocol_version: 0,  // Current version
            encrypted_packet: 0,  // Not exposing encryption in builder
            encryption_algorithm: 0,
            pts_adjustment: self.pts_adjustment,
            cw_index: 0,  // Not exposing encryption
            tier: self.tier,
            splice_command_length,
            splice_command_type: (&splice_command).into(),
            splice_command,
            descriptor_loop_length,
            splice_descriptors: self.descriptors,
            alignment_stuffing_bits: Vec::new(),  // Calculated during serialization
            e_crc_32: None,  // Not exposing encryption
            crc_32: 0,  // Calculated during serialization
        })
    }
}
```

### 2. SpliceInsertBuilder

Builder for the most common splice command - ad insertion.

```rust
pub struct SpliceInsertBuilder {
    splice_event_id: Option<u32>,
    out_of_network: bool,
    program_splice: bool,
    splice_immediate: bool,
    splice_time: Option<Duration>,  // Time from start
    components: Vec<ComponentTiming>,
    duration: Option<Duration>,
    auto_return: bool,
    unique_program_id: u16,
    avail_num: u8,
    avails_expected: u8,
}

#[derive(Clone)]
struct ComponentTiming {
    component_tag: u8,
    splice_time: Option<Duration>,
}

impl SpliceInsertBuilder {
    pub fn new(splice_event_id: u32) -> Self {
        Self {
            splice_event_id: Some(splice_event_id),
            out_of_network: true,  // Most common case
            program_splice: true,  // Most common case
            splice_immediate: false,
            splice_time: None,
            components: Vec::new(),
            duration: None,
            auto_return: true,
            unique_program_id: 0,
            avail_num: 0,
            avails_expected: 0,
        }
    }

    pub fn cancel_event(mut self) -> Self {
        self.splice_event_id = None;  // Indicates cancellation
        self
    }

    pub fn out_of_network(mut self, out: bool) -> Self {
        self.out_of_network = out;
        self
    }

    pub fn immediate(mut self) -> Self {
        self.splice_immediate = true;
        self.splice_time = None;
        self
    }

    pub fn at_pts(mut self, pts_time: Duration) -> BuilderResult<Self> {
        self.splice_immediate = false;
        self.splice_time = Some(pts_time);
        Ok(self)
    }

    pub fn component_splice(mut self, components: Vec<(u8, Option<Duration>)>) -> BuilderResult<Self> {
        if components.len() > 255 {
            return Err(BuilderError::InvalidComponentCount { max: 255, actual: components.len() });
        }
        self.program_splice = false;
        self.components = components.into_iter()
            .map(|(tag, time)| ComponentTiming { component_tag: tag, splice_time: time })
            .collect();
        Ok(self)
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn auto_return(mut self, auto_return: bool) -> Self {
        self.auto_return = auto_return;
        self
    }

    pub fn unique_program_id(mut self, id: u16) -> Self {
        self.unique_program_id = id;
        self
    }

    pub fn avail(mut self, num: u8, expected: u8) -> Self {
        self.avail_num = num;
        self.avails_expected = expected;
        self
    }

    pub fn build(self) -> BuilderResult<SpliceInsert> {
        let (splice_event_id, cancel) = match self.splice_event_id {
            Some(id) => (id, 0),
            None => (0, 1),  // Cancellation
        };

        let splice_time = if self.program_splice && !self.splice_immediate {
            let pts = match self.splice_time {
                Some(duration) => {
                    let ticks = duration.to_pts_ticks();
                    if ticks > 0x1_FFFF_FFFF {
                        return Err(BuilderError::DurationTooLarge { field: "splice_time", duration });
                    }
                    Some(ticks)
                }
                None => None,
            };
            Some(SpliceTime {
                time_specified_flag: 1,
                pts_time: pts,
            })
        } else {
            None
        };

        let mut components = Vec::new();
        if !self.program_splice {
            for c in self.components {
                let splice_time = if !self.splice_immediate {
                    let pts = match c.splice_time {
                        Some(duration) => {
                            let ticks = duration.to_pts_ticks();
                            if ticks > 0x1_FFFF_FFFF {
                                return Err(BuilderError::DurationTooLarge { field: "component_splice_time", duration });
                            }
                            Some(ticks)
                        }
                        None => None,
                    };
                    Some(SpliceTime {
                        time_specified_flag: 1,
                        pts_time: pts,
                    })
                } else {
                    None
                };
                components.push(SpliceInsertComponent {
                    component_tag: c.component_tag,
                    splice_time,
                });
            }
        }

        let break_duration = match self.duration {
            Some(duration) => {
                let ticks = duration.to_pts_ticks();
                if ticks > 0x1_FFFF_FFFF {
                    return Err(BuilderError::DurationTooLarge { field: "duration", duration });
                }
                Some(BreakDuration {
                    auto_return: self.auto_return as u8,
                    reserved: 0,
                    duration: ticks,
                })
            }
            None => None,
        };

        Ok(SpliceInsert {
            splice_event_id,
            splice_event_cancel_indicator: cancel,
            reserved: 0,
            out_of_network_indicator: self.out_of_network as u8,
            program_splice_flag: self.program_splice as u8,
            duration_flag: self.duration.is_some() as u8,
            splice_immediate_flag: self.splice_immediate as u8,
            reserved2: 0,
            splice_time,
            component_count: components.len() as u8,
            components,
            break_duration,
            unique_program_id: self.unique_program_id,
            avail_num: self.avail_num,
            avails_expected: self.avails_expected,
        })
    }
}
```

### 3. TimeSignalBuilder

Simple builder for time signal commands.

```rust
pub struct TimeSignalBuilder {
    pts_time: Option<Duration>,
}

impl TimeSignalBuilder {
    pub fn new() -> Self {
        Self { pts_time: None }
    }

    pub fn immediate(self) -> Self {
        self  // No time specified
    }

    pub fn at_pts(mut self, pts_time: Duration) -> BuilderResult<Self> {
        self.pts_time = Some(pts_time);
        Ok(self)
    }

    pub fn build(self) -> BuilderResult<TimeSignal> {
        let pts_time = match self.pts_time {
            Some(duration) => {
                let ticks = duration.to_pts_ticks();
                if ticks > 0x1_FFFF_FFFF {
                    return Err(BuilderError::DurationTooLarge { field: "pts_time", duration });
                }
                Some(ticks)
            }
            None => None,
        };

        Ok(TimeSignal {
            splice_time: SpliceTime {
                time_specified_flag: pts_time.is_some() as u8,
                pts_time,
            },
        })
    }
}
```

### 4. SegmentationDescriptorBuilder

Builder for the most important descriptor type.

```rust
pub struct SegmentationDescriptorBuilder {
    segmentation_event_id: Option<u32>,
    program_segmentation: bool,
    duration: Option<Duration>,
    delivery_restrictions: Option<DeliveryRestrictions>,
    upid: Option<Upid>,
    segmentation_type: SegmentationType,
    segment_num: u8,
    segments_expected: u8,
    sub_segmentation: Option<SubSegmentation>,
}

pub struct DeliveryRestrictions {
    pub web_delivery_allowed: bool,
    pub no_regional_blackout: bool,
    pub archive_allowed: bool,
    pub device_restrictions: DeviceRestrictions,
}

#[derive(Clone, Copy)]
pub enum DeviceRestrictions {
    None,
    RestrictGroup1,
    RestrictGroup2,
    RestrictBoth,
}

pub struct SubSegmentation {
    pub sub_segment_num: u8,
    pub sub_segments_expected: u8,
}

pub enum Upid {
    None,
    AdId(String),  // 12 ASCII characters
    Umid([u8; 32]),
    Isan([u8; 12]),
    Tid(String),   // 12 ASCII characters
    AiringId(u64),
    Eidr([u8; 12]),
    Uri(String),   // Variable length
    Uuid([u8; 16]),
    // ... other UPID types
}

impl SegmentationDescriptorBuilder {
    pub fn new(event_id: u32, segmentation_type: SegmentationType) -> Self {
        Self {
            segmentation_event_id: Some(event_id),
            program_segmentation: true,
            duration: None,
            delivery_restrictions: None,
            upid: None,
            segmentation_type,
            segment_num: 1,
            segments_expected: 1,
            sub_segmentation: None,
        }
    }

    pub fn cancel_event(mut self) -> Self {
        self.segmentation_event_id = None;
        self
    }

    pub fn duration(mut self, duration: Duration) -> BuilderResult<Self> {
        let ticks = duration.to_pts_ticks();
        if ticks > 0x1_FFFF_FFFF {
            return Err(BuilderError::DurationTooLarge { field: "segmentation_duration", duration });
        }
        self.duration = Some(duration);
        Ok(self)
    }

    pub fn no_restrictions(mut self) -> Self {
        self.delivery_restrictions = None;
        self
    }

    pub fn delivery_restrictions(mut self, restrictions: DeliveryRestrictions) -> Self {
        self.delivery_restrictions = Some(restrictions);
        self
    }

    pub fn upid(mut self, upid: Upid) -> BuilderResult<Self> {
        // Validate UPID based on type
        match &upid {
            Upid::AdId(s) | Upid::Tid(s) => {
                if s.len() != 12 {
                    return Err(BuilderError::InvalidUpidLength { expected: 12, actual: s.len() });
                }
            }
            Upid::Uri(s) => {
                if s.is_empty() || s.len() > 255 {
                    return Err(BuilderError::InvalidValue {
                        field: "uri",
                        reason: "URI must be 1-255 bytes".to_string(),
                    });
                }
            }
            _ => {}  // Other types have fixed sizes
        }
        self.upid = Some(upid);
        Ok(self)
    }

    pub fn segment(mut self, num: u8, expected: u8) -> Self {
        self.segment_num = num;
        self.segments_expected = expected;
        self
    }

    pub fn sub_segment(mut self, num: u8, expected: u8) -> Self {
        self.sub_segmentation = Some(SubSegmentation {
            sub_segment_num: num,
            sub_segments_expected: expected,
        });
        self
    }

    pub fn build(self) -> BuilderResult<SegmentationDescriptor> {
        let (event_id, cancel) = match self.segmentation_event_id {
            Some(id) => (id, false),
            None => (0, true),
        };

        let (delivery_not_restricted, web, blackout, archive, device) = 
            match self.delivery_restrictions {
                None => (true, None, None, None, None),
                Some(r) => (false, 
                    Some(r.web_delivery_allowed),
                    Some(r.no_regional_blackout),
                    Some(r.archive_allowed),
                    Some(r.device_restrictions.into())),
            };

        let (upid_type, upid_bytes) = self.upid.unwrap_or(Upid::None).into();

        let duration_ticks = match self.duration {
            Some(duration) => {
                let ticks = duration.to_pts_ticks();
                if ticks > 0x1_FFFF_FFFF {
                    return Err(BuilderError::DurationTooLarge { field: "segmentation_duration", duration });
                }
                Some(ticks)
            }
            None => None,
        };

        Ok(SegmentationDescriptor {
            segmentation_event_id: event_id,
            segmentation_event_cancel_indicator: cancel,
            program_segmentation_flag: self.program_segmentation,
            segmentation_duration_flag: self.duration.is_some(),
            delivery_not_restricted_flag: delivery_not_restricted,
            web_delivery_allowed_flag: web,
            no_regional_blackout_flag: blackout,
            archive_allowed_flag: archive,
            device_restrictions: device,
            segmentation_duration: duration_ticks,
            segmentation_upid_type: upid_type,
            segmentation_upid_length: upid_bytes.len() as u8,
            segmentation_upid: upid_bytes,
            segmentation_type_id: self.segmentation_type.id(),
            segmentation_type: self.segmentation_type,
            segment_num: self.segment_num,
            segments_expected: self.segments_expected,
            sub_segment_num: self.sub_segmentation.as_ref().map(|s| s.sub_segment_num),
            sub_segments_expected: self.sub_segmentation.as_ref().map(|s| s.sub_segments_expected),
        })
    }
}
```

## Usage Examples

### Example 1: Creating a Splice Insert for Ad Break

```rust
use scte35::builders::*;
use std::time::Duration;

// Create a 30-second ad break starting 20 seconds from start
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
```

### Example 2: Creating a Time Signal with Segmentation Descriptor

```rust
use scte35::builders::*;
use scte35::SegmentationType;

// Create a program start boundary
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
```

### Example 3: Creating an Immediate Splice Out

```rust
use scte35::builders::*;

// Immediate splice out to ads
let section = SpliceInfoSectionBuilder::new()
    .splice_insert(
        SpliceInsertBuilder::new(9999)
            .immediate()
            .out_of_network(true)
            .build()?
    )
    .build()?;
```

### Example 4: Component-Level Splice

```rust
use scte35::builders::*;

// Splice specific audio/video components at 10 seconds
let splice_insert = SpliceInsertBuilder::new(3333)
    .component_splice(vec![
        (0x01, Some(Duration::from_secs(10))),  // Video component
        (0x02, Some(Duration::from_secs(10))),  // Audio component 1
        (0x03, Some(Duration::from_secs(10))),  // Audio component 2
    ])?
    .duration(Duration::from_secs(15))
    .build()?;

let section = SpliceInfoSectionBuilder::new()
    .splice_insert(splice_insert)
    .build()?;
```

### Example 5: Complex Segmentation with Delivery Restrictions

```rust
use scte35::builders::*;
use scte35::SegmentationType;

let restrictions = DeliveryRestrictions {
    web_delivery_allowed: false,
    no_regional_blackout: false,
    archive_allowed: true,
    device_restrictions: DeviceRestrictions::RestrictGroup1,
};

let segmentation = SegmentationDescriptorBuilder::new(
        7777,
        SegmentationType::DistributorAdvertisementStart
    )
    .delivery_restrictions(restrictions)
    .upid(Upid::Uuid([
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
    ]))?
    .segment(2, 6)  // 2nd of 6 segments
    .build()?;

let section = SpliceInfoSectionBuilder::new()
    .time_signal(TimeSignalBuilder::new().at_pts(Duration::from_secs(30))?.build()?)
    .add_segmentation_descriptor(segmentation)
    .tier(0x100)  // Specific tier
    .build()?;
```

## Trait Implementations

```rust
// Add encoded_length() method to SpliceCommand
impl SpliceCommand {
    pub fn encoded_length(&self) -> u16 {
        match self {
            SpliceCommand::SpliceNull => 0,
            SpliceCommand::SpliceInsert(insert) => {
                // Base: 14 bytes
                let mut len = 14;
                if insert.program_splice_flag == 1 && insert.splice_immediate_flag == 0 {
                    len += 5;  // splice_time
                }
                if insert.program_splice_flag == 0 {
                    len += 1;  // component_count
                    len += insert.components.len() * 6;  // each component
                }
                if insert.duration_flag == 1 {
                    len += 5;  // break_duration
                }
                len as u16
            }
            SpliceCommand::TimeSignal(_) => 5,
            SpliceCommand::BandwidthReservation(_) => 4,
            SpliceCommand::SpliceSchedule(_) => {
                // Implementation depends on schedule structure
                todo!("Calculate SpliceSchedule length")
            }
            SpliceCommand::PrivateCommand(pc) => pc.private_command_length as u16 + 3,
            SpliceCommand::Unknown => 0,
        }
    }
}

// Convert SpliceCommand reference to command type byte
impl From<&SpliceCommand> for u8 {
    fn from(command: &SpliceCommand) -> Self {
        match command {
            SpliceCommand::SpliceNull => 0x00,
            SpliceCommand::SpliceSchedule(_) => 0x04,
            SpliceCommand::SpliceInsert(_) => 0x05,
            SpliceCommand::TimeSignal(_) => 0x06,
            SpliceCommand::BandwidthReservation(_) => 0x07,
            SpliceCommand::PrivateCommand(_) => 0xFF,
            SpliceCommand::Unknown => 0xFF,
        }
    }
}

// Convert DeviceRestrictions to u8
impl From<DeviceRestrictions> for u8 {
    fn from(restrictions: DeviceRestrictions) -> Self {
        match restrictions {
            DeviceRestrictions::None => 0x00,
            DeviceRestrictions::RestrictGroup1 => 0x01,
            DeviceRestrictions::RestrictGroup2 => 0x02,
            DeviceRestrictions::RestrictBoth => 0x03,
        }
    }
}

// Convert Upid to (SegmentationUpidType, Vec<u8>)
impl From<Upid> for (SegmentationUpidType, Vec<u8>) {
    fn from(upid: Upid) -> Self {
        match upid {
            Upid::None => (SegmentationUpidType::NotUsed, vec![]),
            Upid::AdId(s) => (SegmentationUpidType::AdID, s.into_bytes()),
            Upid::Umid(bytes) => (SegmentationUpidType::UMID, bytes.to_vec()),
            Upid::Isan(bytes) => (SegmentationUpidType::ISAN, bytes.to_vec()),
            Upid::Tid(s) => (SegmentationUpidType::TID, s.into_bytes()),
            Upid::AiringId(id) => (SegmentationUpidType::AiringID, id.to_be_bytes().to_vec()),
            Upid::Eidr(bytes) => (SegmentationUpidType::EIDR, bytes.to_vec()),
            Upid::Uri(s) => (SegmentationUpidType::URI, s.into_bytes()),
            Upid::Uuid(bytes) => (SegmentationUpidType::UUID, bytes.to_vec()),
        }
    }
}
```

## Implementation Priority

1. **Phase 1**: Core builders
   - `SpliceInfoSectionBuilder`
   - `SpliceInsertBuilder`
   - `TimeSignalBuilder`
   - `SegmentationDescriptorBuilder`

2. **Phase 2**: Additional commands
   - `SpliceScheduleBuilder`
   - `BandwidthReservationBuilder`
   - `PrivateCommandBuilder`

3. **Phase 3**: Helper builders
   - Additional descriptor builders
   - Convenience methods for common patterns

## Testing Strategy

1. **Unit tests** for each builder verifying:
   - Default values are correct
   - Validation works properly
   - All fields can be set correctly

2. **Integration tests** verifying:
   - Complete messages can be built
   - Built messages can be serialized and parsed back
   - CRC calculation works on built messages

3. **Doc tests** for all examples in this document

## Notes

- All builders consume `self` to prevent reuse after `build()`
- PTS times are automatically masked to 33 bits
- Tier values are automatically masked to 12 bits
- Component counts are validated to not exceed 255
- UPID lengths are validated based on type
- Duration values are checked to not exceed 33-bit tick representation