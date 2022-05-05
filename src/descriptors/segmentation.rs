use crate::{BytesWritten, ClockTimeExt, CueError, TransportPacketWrite};
use anyhow::Context;
use ascii::AsciiString;
use bitstream_io::{BigEndian, BitRecorder, BitWrite, BitWriter};
use std::ffi::CStr;
use std::io::Write;
use std::{fmt, io};

use crate::descriptors::{SpliceDescriptorExt, SpliceDescriptorTag};
#[cfg(feature = "serde")]
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SegmentationDescriptor {
    segmentation_event_id: u32,
    segmentation_event_cancel_indicator: bool,
    program_segmentation_flag: bool,
    segmentation_duration_flag: bool,
    delivery_not_restricted_flag: bool,
    web_delivery_allowed_flag: bool,
    no_regional_blackout_flag: bool,
    archive_allowed_flag: bool,
    device_restrictions: DeviceRestrictions,
    components: Vec<Component>,
    segmentation_duration: u64,
    segmentation_upid: SegmentationUpid,
    segmentation_type: SegmentationType,
    segment_num: u8,
    segments_expected: u8,
    sub_segment_num: u8,
    sub_segments_expected: u8,

    descriptor_length: Option<u8>,
}

#[cfg(feature = "serde")]
mod serde_serialization {
    use super::*;
    use crate::ticks_to_secs;
    use crate::time::format_duration;
    use ascii::AsciiStr;
    use serde::ser::{Error, Serialize, SerializeStruct, Serializer};
    use std::fmt::{format, LowerHex};
    use std::time::Duration;

    impl Serialize for SegmentationDescriptor {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use SegmentationFieldSyntax::*;

            #[inline]
            fn as_hex<T>(value: T) -> String
            where
                T: LowerHex,
            {
                format!("0x{:02x}", value)
            }

            let segmentation_syntax = self.segmentation_type.syntax();

            // predict number of fields in the struct
            let mut num_fields = 6;
            if !self.segmentation_event_cancel_indicator {
                num_fields += 12;
                if !self.delivery_not_restricted_flag {
                    num_fields += 4;
                }
                if self.segmentation_duration_flag {
                    num_fields += 2;
                }
                match segmentation_syntax.sub_segment_num {
                    Fixed(_) | NonZero | ZeroOrNonZero => {
                        num_fields += 1;
                    }
                    NotUsed => {}
                }
                match segmentation_syntax.sub_segments_expected {
                    Fixed(_) | NonZero | ZeroOrNonZero => {
                        num_fields += 1;
                    }
                    NotUsed => {}
                }
            }

            let mut state = serializer.serialize_struct("SegmentationDescriptor", num_fields)?;
            state.serialize_field("name", "Segmentation Descriptor")?;
            state.serialize_field(
                "splice_descriptor_tag",
                &as_hex(self.splice_descriptor_tag()),
            )?;
            state.serialize_field("descriptor_length", &self.descriptor_length)?;
            let id = self.identifier().to_be_bytes();
            state.serialize_field(
                "identifier",
                AsciiStr::from_ascii(id.as_slice())
                    .expect("ascii characters")
                    .as_str(),
            )?;
            state.serialize_field("segmentation_event_id", &as_hex(self.segmentation_event_id))?;
            state.serialize_field(
                "segmentation_event_cancel_indicator",
                &self.segmentation_event_cancel_indicator,
            )?;

            if !self.segmentation_event_cancel_indicator {
                state.serialize_field(
                    "program_segmentation_flag",
                    &self.program_segmentation_flag,
                )?;
                state.serialize_field(
                    "segmentation_duration_flag",
                    &self.segmentation_duration_flag,
                )?;
                state.serialize_field(
                    "delivery_not_restricted_flag",
                    &self.delivery_not_restricted_flag,
                )?;
                if !self.delivery_not_restricted_flag {
                    state.serialize_field(
                        "web_delivery_allowed_flag",
                        &self.web_delivery_allowed_flag,
                    )?;
                    state.serialize_field(
                        "no_regional_blackout_flag",
                        &self.no_regional_blackout_flag,
                    )?;
                    state.serialize_field("archive_allowed_flag", &self.archive_allowed_flag)?;
                    state.serialize_field("device_restrictions", &self.device_restrictions)?;
                }
                state.serialize_field("components", &self.components)?;
                if self.segmentation_duration_flag {
                    let duration_secs = ticks_to_secs(self.segmentation_duration);
                    state.serialize_field("segmentation_duration", &self.segmentation_duration)?;
                    state.serialize_field("segmentation_duration_secs", &duration_secs)?;
                    state.serialize_field(
                        "segmentation_duration_human",
                        &format_duration(Duration::from_secs_f64(duration_secs)).to_string(),
                    )?;
                }
                state.serialize_field(
                    "segmentation_upid_type",
                    &as_hex(u8::from(self.segmentation_upid.segmentation_upid_type())),
                )?;
                state.serialize_field(
                    "segmentation_upid_type_name",
                    &format!("{}", self.segmentation_upid.segmentation_upid_type()),
                )?;
                state.serialize_field(
                    "segmentation_upid_length",
                    &self.segmentation_upid.segmentation_upid_length(),
                )?;
                state.serialize_field("segmentation_upid", &self.segmentation_upid)?;
                state.serialize_field(
                    "segmentation_message",
                    &format!("{}", self.segmentation_type),
                )?;
                state.serialize_field("segmentation_type_id", &self.segmentation_type.id())?;
                state.serialize_field("segment_num", &self.segment_num)?;
                state.serialize_field("segments_expected", &self.segments_expected)?;
                match segmentation_syntax.sub_segment_num {
                    Fixed(v) => {
                        state.serialize_field("sub_segment_num", &v)?;
                    }
                    NonZero | ZeroOrNonZero => {
                        state.serialize_field("sub_segment_num", &self.sub_segment_num)?;
                    }
                    NotUsed => {}
                }
                match segmentation_syntax.sub_segments_expected {
                    Fixed(v) => {
                        state.serialize_field("sub_segments_expected", &v)?;
                    }
                    NonZero | ZeroOrNonZero => {
                        state.serialize_field(
                            "sub_segments_expected",
                            &self.sub_segments_expected,
                        )?;
                    }
                    NotUsed => {}
                }
            }
            state.end()
        }
    }

    impl Serialize for SegmentationUpid {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use SegmentationUpid::*;

            let mut recorder = BitRecorder::<u32, BigEndian>::new();
            self.write_to(&mut recorder)
                .map_err(|err| S::Error::custom(format!("{}", err)))?;

            let mut data = Vec::new();
            let mut buffer = BitWriter::endian(&mut data, BigEndian);
            recorder
                .playback(&mut buffer)
                .map_err(|err| S::Error::custom(format!("{}", err)))?;

            // TODO: serialize as struct when variant is MPU and MID
            match self {
                // if field is represented as a character, then show with textual representation
                ISCI(v) | AdID(v) | TID(v) | ADSInformation(v) | URI(v) | SCR(v) => {
                    serializer.serialize_str(v.as_str())
                }
                // if field is represented as a number, then show as hex
                ISAN(v) | EIDR(v) | UUID(v) => serializer.serialize_str(&format!("0x{:x}", v)),
                ISANDeprecated(v) | AiringID(v) => serializer.serialize_str(&format!("0x{:x}", v)),
                // everything else show as hex, we skip the first byte (which is the length)
                _ => serializer.serialize_str(&format!("0x{}", hex::encode(&data[1..]))),
            }
        }
    }
}

impl SegmentationDescriptor {
    pub fn set_segmentation_event_id(&mut self, segmentation_event_id: u32) {
        self.segmentation_event_id = segmentation_event_id;
    }

    pub fn set_segmentation_event_cancel_indicator(
        &mut self,
        segmentation_event_cancel_indicator: bool,
    ) {
        self.segmentation_event_cancel_indicator = segmentation_event_cancel_indicator;
    }

    pub fn set_program_segmentation_flag(&mut self, program_segmentation_flag: bool) {
        self.program_segmentation_flag = program_segmentation_flag;
    }

    pub fn set_segmentation_duration_flag(&mut self, segmentation_duration_flag: bool) {
        self.segmentation_duration_flag = segmentation_duration_flag;
    }

    pub fn set_delivery_not_restricted_flag(&mut self, delivery_not_restricted_flag: bool) {
        self.delivery_not_restricted_flag = delivery_not_restricted_flag;
    }

    pub fn set_web_delivery_allowed_flag(&mut self, web_delivery_allowed_flag: bool) {
        self.web_delivery_allowed_flag = web_delivery_allowed_flag;
    }

    pub fn set_no_regional_blackout_flag(&mut self, no_regional_blackout_flag: bool) {
        self.no_regional_blackout_flag = no_regional_blackout_flag;
    }

    pub fn set_archive_allowed_flag(&mut self, archive_allowed_flag: bool) {
        self.archive_allowed_flag = archive_allowed_flag;
    }

    pub fn set_device_restrictions(&mut self, device_restrictions: DeviceRestrictions) {
        self.device_restrictions = device_restrictions;
    }

    pub fn set_segmentation_duration(&mut self, segmentation_duration: impl ClockTimeExt) {
        self.set_segmentation_duration_flag(true);
        self.segmentation_duration = segmentation_duration.to_90k();
    }

    pub fn set_segmentation_upid(&mut self, segmentation_upid: SegmentationUpid) {
        self.segmentation_upid = segmentation_upid;
    }

    pub fn set_segmentation_type(&mut self, segmentation_type: SegmentationType) {
        self.segmentation_type = segmentation_type;
    }

    pub fn set_segment_num(&mut self, segment_num: u8) {
        self.segment_num = segment_num;
    }

    pub fn set_segments_expected(&mut self, segments_expected: u8) {
        self.segments_expected = segments_expected;
    }

    pub fn set_sub_segment_num(&mut self, sub_segment_num: u8) {
        self.sub_segment_num = sub_segment_num;
    }

    pub fn set_sub_segments_expected(&mut self, sub_segments_expected: u8) {
        self.sub_segments_expected = sub_segments_expected;
    }

    pub(crate) fn write_to<W>(&mut self, buffer: &mut W) -> anyhow::Result<u32>
    where
        W: io::Write,
    {
        use SegmentationFieldSyntax::*;

        let mut recorder: BitRecorder<u32, BigEndian> = BitRecorder::new();
        recorder.write(32, self.identifier())?;
        recorder.write(32, self.segmentation_event_id)?;
        recorder.write_bit(self.segmentation_event_cancel_indicator)?;
        recorder.write(7, 0x7f)?;
        if !self.segmentation_event_cancel_indicator {
            recorder.write_bit(self.program_segmentation_flag)?;
            recorder.write_bit(self.segmentation_duration_flag)?;
            recorder.write_bit(self.delivery_not_restricted_flag)?;
            if !self.delivery_not_restricted_flag {
                recorder.write_bit(self.web_delivery_allowed_flag)?;
                recorder.write_bit(self.no_regional_blackout_flag)?;
                recorder.write_bit(self.archive_allowed_flag)?;
                recorder.write(2, self.device_restrictions as u8)?;
            } else {
                recorder.write(5, 0x1f)?;
            }
            if !self.program_segmentation_flag {
                recorder.write(8, self.components.len() as u8)?;
                for component in &self.components {
                    component.write_to(&mut recorder)?;
                }
            }
            if self.segmentation_duration_flag {
                recorder.write(40, self.segmentation_duration)?;
            }
            recorder.write(8, u8::from(self.segmentation_upid.segmentation_upid_type()))?;
            self.segmentation_upid.write_to(&mut recorder)?;
            recorder.write(8, self.segmentation_type.id())?;

            let s = self.segmentation_type.syntax();
            match s.segment_num {
                Fixed(n) => recorder.write(8, n)?,
                NonZero | ZeroOrNonZero => recorder.write(8, self.segment_num)?, // needs to check for non-zero
                NotUsed => recorder.write(8, 0u8)?,
            }
            match s.segments_expected {
                Fixed(n) => recorder.write(8, n)?,
                NonZero | ZeroOrNonZero => recorder.write(8, self.segments_expected)?, // needs to check for non-zero
                NotUsed => recorder.write(8, 0u8)?,
            }
            match s.sub_segment_num {
                Fixed(n) => recorder.write(8, n)?,
                NonZero | ZeroOrNonZero => recorder.write(8, self.sub_segment_num)?, // needs to check for non-zero
                NotUsed => {}
            }
            match s.sub_segments_expected {
                Fixed(n) => recorder.write(8, n)?,
                NonZero | ZeroOrNonZero => recorder.write(8, self.sub_segments_expected)?, // needs to check for non-zero
                NotUsed => {}
            }
        }

        let descriptor_length = recorder.bytes_written() as u8;

        // Actually write to the output buffer, now we know the total size we need to write out
        let mut buffer = BitWriter::endian(buffer, BigEndian);
        buffer.write(8, self.splice_descriptor_tag())?;
        buffer.write(8, descriptor_length)?;
        recorder.playback(&mut buffer)?;

        // This field is used when serializing the Segmentation Descriptor with serde
        self.descriptor_length = Some(descriptor_length);

        // This is the full size of the descriptor, which includes the 2 bytes of the tag and the length
        Ok(descriptor_length as u32 + 2)
    }
}

impl SpliceDescriptorExt for SegmentationDescriptor {
    fn splice_descriptor_tag(&self) -> u8 {
        SpliceDescriptorTag::Segmentation.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[repr(u8)]
pub enum DeviceRestrictions {
    /// This Segment is restricted for a class of devices defined by an out of band message that
    /// describes which devices are excluded.
    RestrictGroup0 = 0b00,

    /// This Segment is restricted for a class of devices defined by an out of band message that
    /// describes which devices are excluded.
    RestrictGroup1 = 0b01,

    /// This Segment is restricted for a class of devices defined by an out of band message that
    /// describes which devices are excluded.
    RestrictGroup2 = 0b10,

    /// This Segment has no device restrictions.
    None = 0b11,
}

impl Default for DeviceRestrictions {
    fn default() -> Self {
        DeviceRestrictions::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum SegmentationUpidType {
    NotUsed,
    UserDefinedDeprecated,
    ISCI,
    AdID,
    UMID,
    ISANDeprecated,
    ISAN,
    TID,
    AiringID,
    ADI,
    EIDR,
    ATSCContentIdentifier,
    MPU,
    MID,
    ADSInformation,
    URI,
    UUID,
    SCR,
    Reserved(u8),
}

impl Default for SegmentationUpidType {
    fn default() -> Self {
        SegmentationUpidType::NotUsed
    }
}

impl From<SegmentationUpidType> for u8 {
    fn from(s: SegmentationUpidType) -> Self {
        use SegmentationUpidType::*;
        match s {
            NotUsed => 0x00,
            UserDefinedDeprecated => 0x01,
            ISCI => 0x02,
            AdID => 0x03,
            UMID => 0x04,
            ISANDeprecated => 0x05,
            ISAN => 0x06,
            TID => 0x07,
            AiringID => 0x08,
            ADI => 0x09,
            EIDR => 0x0A,
            ATSCContentIdentifier => 0x0B,
            MPU => 0x0C,
            MID => 0x0D,
            ADSInformation => 0x0E,
            URI => 0x0F,
            UUID => 0x10,
            SCR => 0x11,
            Reserved(x) => x,
        }
    }
}

impl fmt::Display for SegmentationUpidType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        use SegmentationUpidType::*;
        match self {
            NotUsed => write!(f, "Not Used"),
            UserDefinedDeprecated => write!(f, "User Defined Deprecated"),
            ISCI => write!(f, "ISCI"),
            AdID => write!(f, "Ad-ID"),
            UMID => write!(f, "UMID"),
            ISANDeprecated => write!(f, "ISAN Deprecated"),
            ISAN => write!(f, "ISAN"),
            TID => write!(f, "TID"),
            AiringID => write!(f, "AiringID"),
            ADI => write!(f, "ADI"),
            EIDR => write!(f, "EIDR"),
            ATSCContentIdentifier => write!(f, "ATSC Content Identifier"),
            MPU => write!(f, "MPU()"),
            MID => write!(f, "MID()"),
            ADSInformation => write!(f, "ADS Information"),
            URI => write!(f, "URI"),
            UUID => write!(f, "UUID"),
            SCR => write!(f, "SCR"),
            Reserved(_) => write!(f, "Reserved"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SegmentationUpid {
    NotUsed,
    UserDefinedDeprecated(AsciiString),
    ISCI(AsciiString),
    AdID(AsciiString),
    UMID(AsciiString),
    ISANDeprecated(u64),
    ISAN(u128),
    TID(AsciiString),
    AiringID(u64),
    ADI(AsciiString),
    EIDR(u128),
    ATSCContentIdentifier(AsciiString),
    MPU,
    MID,
    ADSInformation(AsciiString),
    URI(AsciiString),
    UUID(u128),
    SCR(AsciiString),
    Reserved(u8),
}

impl Default for SegmentationUpid {
    fn default() -> Self {
        SegmentationUpid::NotUsed
    }
}

impl SegmentationUpid {
    pub fn segmentation_upid_type(&self) -> SegmentationUpidType {
        use SegmentationUpid::*;
        match self {
            NotUsed => SegmentationUpidType::NotUsed,
            UserDefinedDeprecated(_) => SegmentationUpidType::UserDefinedDeprecated,
            ISCI(_) => SegmentationUpidType::ISCI,
            AdID(_) => SegmentationUpidType::AdID,
            UMID(_) => SegmentationUpidType::UMID,
            ISANDeprecated(_) => SegmentationUpidType::ISANDeprecated,
            ISAN(_) => SegmentationUpidType::ISAN,
            TID(_) => SegmentationUpidType::TID,
            AiringID(_) => SegmentationUpidType::AiringID,
            ADI(_) => SegmentationUpidType::ADI,
            EIDR(_) => SegmentationUpidType::EIDR,
            ATSCContentIdentifier(_) => SegmentationUpidType::ATSCContentIdentifier,
            MPU => SegmentationUpidType::MPU,
            MID => SegmentationUpidType::MID,
            ADSInformation(_) => SegmentationUpidType::ADSInformation,
            URI(_) => SegmentationUpidType::URI,
            UUID(_) => SegmentationUpidType::UUID,
            SCR(_) => SegmentationUpidType::SCR,
            Reserved(r) => SegmentationUpidType::Reserved(*r),
        }
    }

    fn segmentation_upid_length(&self) -> u8 {
        use SegmentationUpid::*;
        match self {
            NotUsed => 0,
            UserDefinedDeprecated(s) => s.len() as u8,
            ISCI(s) => 8,
            AdID(s) => 12,
            UMID(s) => 32,
            ISANDeprecated(_) => 8,
            ISAN(_) => 12,
            TID(s) => 8,
            AiringID(_) => 8,
            ADI(s) => s.len() as u8,
            EIDR(_) => 12,
            ATSCContentIdentifier(s) => s.len() as u8,
            MPU => 0,
            MID => 0,
            ADSInformation(s) => s.len() as u8,
            URI(s) => s.len() as u8,
            UUID(_) => 16,
            SCR(s) => s.len() as u8,
            Reserved(_) => 0,
        }
    }

    fn write_to(&self, out: &mut BitRecorder<u32, BigEndian>) -> anyhow::Result<()> {
        use SegmentationUpid::*;

        let mut recorder = BitRecorder::<u32, BigEndian>::new();

        match self {
            AiringID(v) | ISANDeprecated(v) => {
                // 8 byes is 64 bits
                recorder.write(64, *v)?
            }
            ISAN(value) | EIDR(value) | UUID(value) => {
                recorder.write_bytes(value.to_be_bytes().as_slice())?
            }
            UserDefinedDeprecated(value)
            | ADI(value)
            | ATSCContentIdentifier(value)
            | ADSInformation(value)
            | URI(value)
            | SCR(value) => recorder.write_bytes(value.as_bytes())?,
            ISCI(v) => {
                let buf = v.as_bytes().iter().take(8).copied().collect::<Vec<_>>();
                recorder.write_bytes(buf.as_slice())?;
            }
            AdID(v) | TID(v) => {
                let buf = v.as_bytes().iter().take(12).copied().collect::<Vec<_>>();
                recorder.write_bytes(buf.as_slice())?;
            }
            UMID(v) => {
                let buf = v.as_bytes().iter().take(32).copied().collect::<Vec<_>>();
                recorder.write_bytes(buf.as_slice())?;
            }
            MPU => todo!("Needs to implement MPU() record"),
            MID => todo!("Needs to implement MID() record"),
            NotUsed => {}
            Reserved(_) => {}
        }

        match self {
            NotUsed | Reserved(_) => {
                out.write(8, 0u8)?;
            }
            // All variants with any contained value use the same logic
            _ => {
                out.write(8, recorder.bytes_written() as u8)?;
                recorder.playback(out)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
struct Component {
    component_tag: u8,
    pts_offset: u64,
}

impl Component {
    fn write_to(&self, recorder: &mut BitRecorder<u32, BigEndian>) -> io::Result<()> {
        recorder.write(8, self.component_tag)?;
        recorder.write(7, 0x7f)?;
        recorder.write(33, self.pts_offset)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum SegmentationType {
    NotIndicated,
    ContentIdentification,
    ProgramStart,
    ProgramEnd,
    ProgramEarlyTermination,
    ProgramBreakaway,
    ProgramResumption,
    ProgramRunoverPlanned,
    ProgramRunoverUnplanned,
    ProgramOverlapStart,
    ProgramBlackoutOverride,
    ProgramJoin,
    ChapterStart,
    ChapterEnd,
    BreakStart,
    BreakEnd,
    OpeningCreditStartDeprecated,
    OpeningCreditEndDeprecated,
    ClosingCreditStartDeprecated,
    ClosingCreditEndDeprecated,
    ProviderAdvertisementStart,
    ProviderAdvertisementEnd,
    DistributorAdvertisementStart,
    DistributorAdvertisementEnd,
    ProviderPlacementOpportunityStart,
    ProviderPlacementOpportunityEnd,
    DistributorPlacementOpportunityStart,
    DistributorPlacementOpportunityEnd,
    ProviderOverlayPlacementOpportunityStart,
    ProviderOverlayPlacementOpportunityEnd,
    DistributorOverlayPlacementOpportunityStart,
    DistributorOverlayPlacementOpportunityEnd,
    ProviderPromoStart,
    ProviderPromoEnd,
    DistributorPromoStart,
    DistributorPromoEnd,
    UnscheduledEventStart,
    UnscheduledEventEnd,
    AlternateContentOpportunityStart,
    AlternateContentOpportunityEnd,
    ProviderAdBlockStart,
    ProviderAdBlockEnd,
    DistributorAdBlockStart,
    DistributorAdBlockEnd,
    NetworkStart,
    NetworkEnd,
}

impl Default for SegmentationType {
    fn default() -> Self {
        SegmentationType::NotIndicated
    }
}

impl SegmentationType {
    fn id(&self) -> u8 {
        use SegmentationType::*;
        match self {
            NotIndicated => 0x00,
            ContentIdentification => 0x01,
            ProgramStart => 0x10,
            ProgramEnd => 0x11,
            ProgramEarlyTermination => 0x12,
            ProgramBreakaway => 0x13,
            ProgramResumption => 0x14,
            ProgramRunoverPlanned => 0x15,
            ProgramRunoverUnplanned => 0x16,
            ProgramOverlapStart => 0x17,
            ProgramBlackoutOverride => 0x18,
            ProgramJoin => 0x19,
            ChapterStart => 0x20,
            ChapterEnd => 0x21,
            BreakStart => 0x22,
            BreakEnd => 0x23,
            OpeningCreditStartDeprecated => 0x24,
            OpeningCreditEndDeprecated => 0x25,
            ClosingCreditStartDeprecated => 0x26,
            ClosingCreditEndDeprecated => 0x27,
            ProviderAdvertisementStart => 0x30,
            ProviderAdvertisementEnd => 0x31,
            DistributorAdvertisementStart => 0x32,
            DistributorAdvertisementEnd => 0x33,
            ProviderPlacementOpportunityStart => 0x34,
            ProviderPlacementOpportunityEnd => 0x35,
            DistributorPlacementOpportunityStart => 0x36,
            DistributorPlacementOpportunityEnd => 0x37,
            ProviderOverlayPlacementOpportunityStart => 0x38,
            ProviderOverlayPlacementOpportunityEnd => 0x39,
            DistributorOverlayPlacementOpportunityStart => 0x3A,
            DistributorOverlayPlacementOpportunityEnd => 0x3B,
            ProviderPromoStart => 0x3C,
            ProviderPromoEnd => 0x3D,
            DistributorPromoStart => 0x3E,
            DistributorPromoEnd => 0x3F,
            UnscheduledEventStart => 0x40,
            UnscheduledEventEnd => 0x41,
            AlternateContentOpportunityStart => 0x42,
            AlternateContentOpportunityEnd => 0x43,
            ProviderAdBlockStart => 0x44,
            ProviderAdBlockEnd => 0x45,
            DistributorAdBlockStart => 0x46,
            DistributorAdBlockEnd => 0x47,
            NetworkStart => 0x50,
            NetworkEnd => 0x51,
        }
    }

    /// Reflects definitions on the Table 23 of the spec.
    fn syntax(&self) -> SegmentationTypeSyntax {
        use SegmentationFieldSyntax::*;
        use SegmentationType::*;

        match self {
            NotIndicated => SegmentationTypeSyntax {
                segment_num: Fixed(0),
                segments_expected: Fixed(0),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ContentIdentification => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramStart => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramEnd => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramEarlyTermination => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramBreakaway => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramResumption => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramRunoverPlanned => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramRunoverUnplanned => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramOverlapStart => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramBlackoutOverride => SegmentationTypeSyntax {
                segment_num: Fixed(0),
                segments_expected: Fixed(0),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProgramJoin => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ChapterStart => SegmentationTypeSyntax {
                segment_num: NonZero,
                segments_expected: NonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ChapterEnd => SegmentationTypeSyntax {
                segment_num: NonZero,
                segments_expected: NonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            BreakStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            BreakEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            OpeningCreditStartDeprecated => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            OpeningCreditEndDeprecated => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ClosingCreditStartDeprecated => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ClosingCreditEndDeprecated => SegmentationTypeSyntax {
                segment_num: Fixed(1),
                segments_expected: Fixed(1),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProviderAdvertisementStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProviderAdvertisementEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            DistributorAdvertisementStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            DistributorAdvertisementEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProviderPlacementOpportunityStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: ZeroOrNonZero,
                sub_segments_expected: ZeroOrNonZero,
            },
            ProviderPlacementOpportunityEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            DistributorPlacementOpportunityStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: ZeroOrNonZero,
                sub_segments_expected: ZeroOrNonZero,
            },
            DistributorPlacementOpportunityEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProviderOverlayPlacementOpportunityStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: ZeroOrNonZero,
                sub_segments_expected: ZeroOrNonZero,
            },
            ProviderOverlayPlacementOpportunityEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            DistributorOverlayPlacementOpportunityStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: ZeroOrNonZero,
                sub_segments_expected: ZeroOrNonZero,
            },
            DistributorOverlayPlacementOpportunityEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProviderPromoStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProviderPromoEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            DistributorPromoStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            DistributorPromoEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            UnscheduledEventStart => SegmentationTypeSyntax {
                segment_num: Fixed(0),
                segments_expected: Fixed(0),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            UnscheduledEventEnd => SegmentationTypeSyntax {
                segment_num: Fixed(0),
                segments_expected: Fixed(0),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            AlternateContentOpportunityStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            AlternateContentOpportunityEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProviderAdBlockStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            ProviderAdBlockEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            DistributorAdBlockStart => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            DistributorAdBlockEnd => SegmentationTypeSyntax {
                segment_num: ZeroOrNonZero,
                segments_expected: ZeroOrNonZero,
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            NetworkStart => SegmentationTypeSyntax {
                segment_num: Fixed(0),
                segments_expected: Fixed(0),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
            NetworkEnd => SegmentationTypeSyntax {
                segment_num: Fixed(0),
                segments_expected: Fixed(0),
                sub_segment_num: NotUsed,
                sub_segments_expected: NotUsed,
            },
        }
    }
}

impl fmt::Display for SegmentationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use SegmentationType::*;
        match self {
            NotIndicated => write!(f, "Not Indicated"),
            ContentIdentification => write!(f, "Content Identification"),
            ProgramStart => write!(f, "Program Start"),
            ProgramEnd => write!(f, "Program End"),
            ProgramEarlyTermination => write!(f, "Program Early Termination"),
            ProgramBreakaway => write!(f, "Program Breakaway"),
            ProgramResumption => write!(f, "Program Resumption"),
            ProgramRunoverPlanned => write!(f, "Program Runover Planned"),
            ProgramRunoverUnplanned => write!(f, "Program Runover Unplanned"),
            ProgramOverlapStart => write!(f, "Program Overlap Start"),
            ProgramBlackoutOverride => write!(f, "Program Blackout Override"),
            ProgramJoin => write!(f, "Program Join"),
            ChapterStart => write!(f, "Chapter Start"),
            ChapterEnd => write!(f, "Chapter End"),
            BreakStart => write!(f, "Break Start"),
            BreakEnd => write!(f, "Break End"),
            OpeningCreditStartDeprecated => write!(f, "Opening Credit Start (Deprecated)"),
            OpeningCreditEndDeprecated => write!(f, "Opening Credit End (Deprecated)"),
            ClosingCreditStartDeprecated => write!(f, "Closing Credit Start (Deprecated)"),
            ClosingCreditEndDeprecated => write!(f, "Closing Credit End (Deprecated)"),
            ProviderAdvertisementStart => write!(f, "Provider Advertisement Start"),
            ProviderAdvertisementEnd => write!(f, "Provider Advertisement End"),
            DistributorAdvertisementStart => write!(f, "Distributor Advertisement Start"),
            DistributorAdvertisementEnd => write!(f, "Distributor Advertisement End"),
            ProviderPlacementOpportunityStart => write!(f, "Provider Placement Opportunity Start"),
            ProviderPlacementOpportunityEnd => write!(f, "Provider Placement Opportunity End"),
            DistributorPlacementOpportunityStart => {
                write!(f, "Distributor Placement Opportunity Start")
            }
            DistributorPlacementOpportunityEnd => {
                write!(f, "Distributor Placement Opportunity End")
            }
            ProviderOverlayPlacementOpportunityStart => {
                write!(f, "Provider Overlay Placement Opportunity Start")
            }
            ProviderOverlayPlacementOpportunityEnd => {
                write!(f, "Provider Overlay Placement Opportunity End")
            }
            DistributorOverlayPlacementOpportunityStart => {
                write!(f, "Distributor Overlay Placement Opportunity Start")
            }
            DistributorOverlayPlacementOpportunityEnd => {
                write!(f, "Distributor Overlay Placement Opportunity End")
            }
            ProviderPromoStart => write!(f, "Provider Promo Start"),
            ProviderPromoEnd => write!(f, "Provider Promo End"),
            DistributorPromoStart => write!(f, "Distributor Promo Start"),
            DistributorPromoEnd => write!(f, "Distributor Promo End"),
            UnscheduledEventStart => write!(f, "Unscheduled Event Start"),
            UnscheduledEventEnd => write!(f, "Unscheduled Event End"),
            AlternateContentOpportunityStart => write!(f, "Alternate Content Opportunity Start"),
            AlternateContentOpportunityEnd => write!(f, "Alternate Content Opportunity End"),
            ProviderAdBlockStart => write!(f, "Provider Ad Block Start"),
            ProviderAdBlockEnd => write!(f, "Provider Ad Block End"),
            DistributorAdBlockStart => write!(f, "Distributor Ad Block Start"),
            DistributorAdBlockEnd => write!(f, "Distributor Ad Block End"),
            NetworkStart => write!(f, "Network Start"),
            NetworkEnd => write!(f, "Network End"),
        }
    }
}

impl TryFrom<u8> for SegmentationType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use SegmentationType::*;
        match value {
            0x00 => Ok(NotIndicated),
            0x01 => Ok(ContentIdentification),
            0x10 => Ok(ProgramStart),
            0x11 => Ok(ProgramEnd),
            0x12 => Ok(ProgramEarlyTermination),
            0x13 => Ok(ProgramBreakaway),
            0x14 => Ok(ProgramResumption),
            0x15 => Ok(ProgramRunoverPlanned),
            0x16 => Ok(ProgramRunoverUnplanned),
            0x17 => Ok(ProgramOverlapStart),
            0x18 => Ok(ProgramBlackoutOverride),
            0x19 => Ok(ProgramJoin),
            0x20 => Ok(ChapterStart),
            0x21 => Ok(ChapterEnd),
            0x22 => Ok(BreakStart),
            0x23 => Ok(BreakEnd),
            0x24 => Ok(OpeningCreditStartDeprecated),
            0x25 => Ok(OpeningCreditEndDeprecated),
            0x26 => Ok(ClosingCreditStartDeprecated),
            0x27 => Ok(ClosingCreditEndDeprecated),
            0x30 => Ok(ProviderAdvertisementStart),
            0x31 => Ok(ProviderAdvertisementEnd),
            0x32 => Ok(DistributorAdvertisementStart),
            0x33 => Ok(DistributorAdvertisementEnd),
            0x34 => Ok(ProviderPlacementOpportunityStart),
            0x35 => Ok(ProviderPlacementOpportunityEnd),
            0x36 => Ok(DistributorPlacementOpportunityStart),
            0x37 => Ok(DistributorPlacementOpportunityEnd),
            0x38 => Ok(ProviderOverlayPlacementOpportunityStart),
            0x39 => Ok(ProviderOverlayPlacementOpportunityEnd),
            0x3A => Ok(DistributorOverlayPlacementOpportunityStart),
            0x3B => Ok(DistributorOverlayPlacementOpportunityEnd),
            0x3C => Ok(ProviderPromoStart),
            0x3D => Ok(ProviderPromoEnd),
            0x3E => Ok(DistributorPromoStart),
            0x3F => Ok(DistributorPromoEnd),
            0x40 => Ok(UnscheduledEventStart),
            0x41 => Ok(UnscheduledEventEnd),
            0x42 => Ok(AlternateContentOpportunityStart),
            0x43 => Ok(AlternateContentOpportunityEnd),
            0x44 => Ok(ProviderAdBlockStart),
            0x45 => Ok(ProviderAdBlockEnd),
            0x46 => Ok(DistributorAdBlockStart),
            0x47 => Ok(DistributorAdBlockEnd),
            0x50 => Ok(NetworkStart),
            0x51 => Ok(NetworkEnd),
            _ => Err(()),
        }
    }
}

enum SegmentationFieldSyntax {
    Fixed(u8),
    ZeroOrNonZero,
    NonZero,
    NotUsed,
}

struct SegmentationTypeSyntax {
    segment_num: SegmentationFieldSyntax,
    segments_expected: SegmentationFieldSyntax,
    sub_segment_num: SegmentationFieldSyntax,
    sub_segments_expected: SegmentationFieldSyntax,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use assert_json_diff::assert_json_eq;
    use std::time::Duration;

    #[test]
    fn write_segmentation_upid_airing_id() -> Result<()> {
        let mut data = Vec::new();
        let mut buffer = BitWriter::endian(&mut data, BigEndian);
        let mut recorder = BitRecorder::<u32, BigEndian>::new();

        let segmentation_upid = SegmentationUpid::AiringID(0x2ca0a18a);
        segmentation_upid.write_to(&mut recorder)?;

        recorder.playback(&mut buffer)?;

        // length (1 byte) + data (8 bytes)
        assert_eq!(recorder.bytes_written(), 9);

        let hex = hex::encode(data[1..].to_vec());
        assert_eq!(hex, "000000002ca0a18a".to_string());

        Ok(())
    }

    #[test]
    fn write_segmentation_descriptor() -> Result<()> {
        let mut data = Vec::new();
        let mut descriptor = SegmentationDescriptor::default();
        descriptor.set_segmentation_event_id(0x4800008e);
        descriptor.set_program_segmentation_flag(true);
        descriptor.set_segmentation_duration_flag(true);
        descriptor.set_no_regional_blackout_flag(true);
        descriptor.set_archive_allowed_flag(true);
        descriptor.set_segmentation_duration(27630000);
        descriptor.set_segmentation_upid(SegmentationUpid::AiringID(0x2ca0a18a));
        descriptor.set_segmentation_type(SegmentationType::ProviderPlacementOpportunityStart);
        descriptor.set_segment_num(2);
        descriptor.set_sub_segment_num(154);
        descriptor.set_sub_segments_expected(201);

        descriptor.write_to(&mut data)?;

        let hex = hex::encode(data.as_slice());
        assert_eq!(
            hex,
            "021e435545494800008e7fcf0001a599b00808000000002ca0a18a3402009ac9".to_string()
        );

        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize_segmentation_to_json() -> Result<()> {
        let mut descriptor = SegmentationDescriptor::default();
        descriptor.set_segmentation_event_id(0x4800008e);
        descriptor.set_program_segmentation_flag(true);
        descriptor.set_segmentation_duration_flag(true);
        descriptor.set_no_regional_blackout_flag(true);
        descriptor.set_archive_allowed_flag(true);
        descriptor.set_segmentation_duration(Duration::from_secs_f32(307.0));
        descriptor.set_segmentation_duration(27630000);
        descriptor.set_segmentation_upid(SegmentationUpid::AiringID(0x2ca0a18a));
        descriptor.set_segmentation_type(SegmentationType::ProviderPlacementOpportunityStart);
        descriptor.set_segment_num(2);
        descriptor.set_sub_segment_num(154);
        descriptor.set_sub_segments_expected(201);

        // We need to write to calculate the length
        let mut data = Vec::new();
        descriptor.write_to(&mut data)?;

        let expected_json: serde_json::Value = serde_json::from_str(
            r#"{
            "name": "Segmentation Descriptor",
            "splice_descriptor_tag": "0x02",
            "descriptor_length": 30,
            "identifier": "CUEI",
            "segmentation_event_id": "0x4800008e",
            "segmentation_event_cancel_indicator": false,
            "program_segmentation_flag": true,
            "segmentation_duration_flag": true,
            "delivery_not_restricted_flag": false,
            "web_delivery_allowed_flag": false,
            "no_regional_blackout_flag": true,
            "archive_allowed_flag": true,
            "device_restrictions": "None",
            "components": [],
            "segmentation_duration": 27630000,
            "segmentation_duration_secs": 307.0,
            "segmentation_duration_human": "5m 7s",
            "segmentation_upid_type": "0x08",
            "segmentation_upid_type_name": "AiringID",
            "segmentation_upid_length": 8,
            "segmentation_upid": "0x2ca0a18a",
            "segmentation_message": "Provider Placement Opportunity Start",
            "segmentation_type_id": 52,
            "segment_num": 2,
            "segments_expected": 0,
            "sub_segment_num": 154,
            "sub_segments_expected": 201
        }"#,
        )?;

        assert_json_eq!(serde_json::to_value(&descriptor)?, expected_json);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize_segmentation_with_cancel_indicator_to_json() -> Result<()> {
        let mut descriptor = SegmentationDescriptor::default();
        descriptor.set_segmentation_event_id(0x4800008e);
        descriptor.set_segmentation_event_cancel_indicator(true);

        // We need to write to calculate the length
        let mut data = Vec::new();
        descriptor.write_to(&mut data)?;

        let expected_json: serde_json::Value = serde_json::from_str(
            r#"{
            "name": "Segmentation Descriptor",
            "splice_descriptor_tag": "0x02",
            "descriptor_length": 9,
            "identifier": "CUEI",
            "segmentation_event_id": "0x4800008e",
            "segmentation_event_cancel_indicator": true
        }"#,
        )?;

        assert_json_eq!(serde_json::to_value(&descriptor)?, expected_json);
        Ok(())
    }
}
