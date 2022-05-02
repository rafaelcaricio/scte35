use crate::{CueError, TransportPacketWrite};
use anyhow::Context;
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::Write;

use crate::descriptors::{SpliceDescriptorExt, SpliceDescriptorTag};
#[cfg(feature = "serde")]
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
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
}

impl TransportPacketWrite for SegmentationDescriptor {
    fn write_to<W>(&self, buffer: &mut W) -> anyhow::Result<()>
    where
        W: io::Write,
    {
        use SegmentationFieldSyntax::*;

        let mut data = Vec::new();
        let mut internal_buffer = BitWriter::endian(&mut data, BigEndian);
        internal_buffer.write(32, self.identifier())?;
        internal_buffer.write(32, self.segmentation_event_id)?;
        internal_buffer.write_bit(self.segmentation_event_cancel_indicator)?;
        internal_buffer.write(7, 0x7f)?;
        if !self.segmentation_event_cancel_indicator {
            internal_buffer.write_bit(self.program_segmentation_flag)?;
            internal_buffer.write_bit(self.segmentation_duration_flag)?;
            internal_buffer.write_bit(self.delivery_not_restricted_flag)?;
            if !self.delivery_not_restricted_flag {
                internal_buffer.write_bit(self.web_delivery_allowed_flag)?;
                internal_buffer.write_bit(self.no_regional_blackout_flag)?;
                internal_buffer.write_bit(self.archive_allowed_flag)?;
                internal_buffer.write(2, self.device_restrictions as u8)?;
            } else {
                internal_buffer.write(5, 0x1f)?;
            }
            if !self.program_segmentation_flag {
                internal_buffer.write(8, self.components.len() as u8)?;
                for component in &self.components {
                    component.write_to(&mut internal_buffer)?;
                }
            }
            if self.segmentation_duration_flag {
                internal_buffer.write(40, self.segmentation_duration)?;
            }
            internal_buffer.write(8, u8::from(self.segmentation_upid.segmentation_upid_type()))?;
            self.segmentation_upid.write_to(&mut internal_buffer)?;
            internal_buffer.write(8, self.segmentation_type.id())?;

            let s = self.segmentation_type.syntax();
            match s.segment_num {
                Fixed(n) => internal_buffer.write(8, n)?,
                NonZero | ZeroOrNonZero => internal_buffer.write(8, self.segment_num)?, // needs to check for non-zero
                NotUsed => internal_buffer.write(8, 0u8)?,
            }
            match s.segments_expected {
                Fixed(n) => internal_buffer.write(8, n)?,
                NonZero | ZeroOrNonZero => internal_buffer.write(8, self.segments_expected)?, // needs to check for non-zero
                NotUsed => internal_buffer.write(8, 0u8)?,
            }
            match s.sub_segment_num {
                Fixed(n) => internal_buffer.write(8, n)?,
                NonZero | ZeroOrNonZero => internal_buffer.write(8, self.sub_segment_num)?, // needs to check for non-zero
                NotUsed => {}
            }
            match s.sub_segments_expected {
                Fixed(n) => internal_buffer.write(8, n)?,
                NonZero | ZeroOrNonZero => internal_buffer.write(8, self.sub_segments_expected)?, // needs to check for non-zero
                NotUsed => {}
            }
        }

        internal_buffer.flush()?;

        let mut buffer = BitWriter::endian(buffer, BigEndian);
        buffer.write(8, self.splice_descriptor_tag())?;
        buffer.write(8, data.len() as u8)?;
        buffer.write_bytes(data.as_slice())?;

        Ok(())
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
enum DeviceRestrictions {
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

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl Display for SegmentationUpidType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
#[cfg_attr(feature = "serde", derive(Serialize))]
#[non_exhaustive]
pub enum SegmentationUpid {
    NotUsed,
    UserDefinedDeprecated,
    ISCI,
    AdID,
    UMID,
    ISANDeprecated,
    ISAN,
    TID,
    AiringID(u64),
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

impl SegmentationUpid {
    pub fn segmentation_upid_type(&self) -> SegmentationUpidType {
        use SegmentationUpid::*;
        match self {
            NotUsed => SegmentationUpidType::NotUsed,
            UserDefinedDeprecated => SegmentationUpidType::UserDefinedDeprecated,
            ISCI => SegmentationUpidType::ISCI,
            AdID => SegmentationUpidType::AdID,
            UMID => SegmentationUpidType::UMID,
            ISANDeprecated => SegmentationUpidType::ISANDeprecated,
            ISAN => SegmentationUpidType::ISAN,
            TID => SegmentationUpidType::TID,
            AiringID(_) => SegmentationUpidType::AiringID,
            ADI => SegmentationUpidType::ADI,
            EIDR => SegmentationUpidType::EIDR,
            ATSCContentIdentifier => SegmentationUpidType::ATSCContentIdentifier,
            MPU => SegmentationUpidType::MPU,
            MID => SegmentationUpidType::MID,
            ADSInformation => SegmentationUpidType::ADSInformation,
            URI => SegmentationUpidType::URI,
            UUID => SegmentationUpidType::UUID,
            SCR => SegmentationUpidType::SCR,
            Reserved(r) => SegmentationUpidType::Reserved(*r),
        }
    }

    fn write_to<W>(&self, out: &mut BitWriter<W, BigEndian>) -> io::Result<()>
    where
        W: io::Write,
    {
        use SegmentationUpid::*;

        let mut data = Vec::new();
        let mut buffer = BitWriter::endian(&mut data, BigEndian);

        match self {
            // URI(uri) => {
            //     let raw_value = CStr::from_bytes_with_nul("https://link.com".as_bytes()).unwrap();
            //     buffer.write_bytes(raw_value.to_bytes())?;
            // }
            AiringID(aid) => {
                // 8 bytes is 64 bits
                buffer.write(64, *aid)?;
            }
            _ => {}
        }

        buffer.flush()?;

        match self {
            // All variants with variable length use the same write logic
            UserDefinedDeprecated | URI | AiringID(_) => {
                out.write(8, data.len() as u8)?;
                out.write_bytes(data.as_slice())?;
            }
            _ => {
                out.write(8, 0x0)?;
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
    fn write_to<W>(&self, buffer: &mut BitWriter<W, BigEndian>) -> io::Result<()>
    where
        W: io::Write,
    {
        buffer.write(8, self.component_tag)?;
        buffer.write(7, 0x7f)?;
        buffer.write(33, self.pts_offset)
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
    use std::time::Duration;

    #[test]
    fn write_segmentation_upid_airing_id() -> Result<()> {
        let mut data = Vec::new();
        let mut buffer = BitWriter::endian(&mut data, BigEndian);

        let segmentation_upid = SegmentationUpid::AiringID(0x2ca0a18a);
        segmentation_upid.write_to(&mut buffer)?;

        // length (1 byte) + data (8 bytes)
        assert_eq!(data.len(), 9);

        let hex = hex::encode(data[1..].to_vec());
        assert_eq!(hex, "000000002ca0a18a".to_string());

        Ok(())
    }

    #[test]
    fn write_segmentation_descriptor() -> Result<()> {
        let mut data = Vec::new();
        let segmentation_descriptor = SegmentationDescriptor {
            segmentation_event_id: 0x4800008e,
            segmentation_event_cancel_indicator: false,
            program_segmentation_flag: true,
            segmentation_duration_flag: true,
            delivery_not_restricted_flag: false,
            web_delivery_allowed_flag: false,
            no_regional_blackout_flag: true,
            archive_allowed_flag: true,
            device_restrictions: DeviceRestrictions::None,
            components: vec![],
            segmentation_duration: 27630000,
            segmentation_upid: SegmentationUpid::AiringID(0x2ca0a18a),
            segmentation_type: SegmentationType::ProviderPlacementOpportunityStart,
            segment_num: 2,
            segments_expected: 0,
            sub_segment_num: 154,
            sub_segments_expected: 201,
        };
        segmentation_descriptor.write_to(&mut data)?;

        let hex = hex::encode(data.as_slice());
        assert_eq!(
            hex,
            "021e435545494800008e7fcf0001a599b00808000000002ca0a18a3402009ac9".to_string()
        );

        Ok(())
    }
}
