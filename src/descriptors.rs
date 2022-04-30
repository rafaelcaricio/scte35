use crate::{CueError, TransportPacketWrite};
use std::io::Write;

pub trait SpliceDescriptor {
    fn splice_descriptor_tag(&self) -> u8;
}

struct SegmentationDescriptor {
    identifier: u32,
    segmentation_event_id: u32,
    segmentation_event_cancel_indicator: bool,
    program_segmentation: Vec<Component>,
    delivery_restricted: Option<DeliveryRestriction>,
    segmentation_duration: Option<u64>,
    segmentation_upid: SegmentationUpid,
}

impl TransportPacketWrite for SegmentationDescriptor {
    fn write_to<W>(&self, buffer: &mut W) -> Result<(), CueError>
    where
        W: Write,
    {
        todo!()
    }
}

impl SpliceDescriptor for SegmentationDescriptor {
    fn splice_descriptor_tag(&self) -> u8 {
        SpliceDescriptorTag::Segmentation.into()
    }
}

struct DeliveryRestriction {
    web_delivery_allowed_flag: bool,
    no_regional_blackout_flag: bool,
    archive_allowed_flag: bool,
    device_restrictions: DeviceRestrictions,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum SpliceDescriptorTag {
    Avail,
    DTMF,
    Segmentation,
    Time,
    Audio,
    Reserved(u8),
    DVB(u8),
}

impl From<u8> for SpliceDescriptorTag {
    fn from(value: u8) -> Self {
        match value {
            0x0 => SpliceDescriptorTag::Avail,
            0x1 => SpliceDescriptorTag::DTMF,
            0x2 => SpliceDescriptorTag::Segmentation,
            0x3 => SpliceDescriptorTag::Time,
            0x4 => SpliceDescriptorTag::Audio,
            0x5..=0xEF => SpliceDescriptorTag::Reserved(value),
            0xF0..=0xFF => SpliceDescriptorTag::DVB(value),
        }
    }
}

impl From<SpliceDescriptorTag> for u8 {
    fn from(value: SpliceDescriptorTag) -> Self {
        match value {
            SpliceDescriptorTag::Avail => 0x0,
            SpliceDescriptorTag::DTMF => 0x1,
            SpliceDescriptorTag::Segmentation => 0x2,
            SpliceDescriptorTag::Time => 0x3,
            SpliceDescriptorTag::Audio => 0x4,
            SpliceDescriptorTag::Reserved(value) => value,
            SpliceDescriptorTag::DVB(value) => value,
        }
    }
}

enum DeviceRestrictions {
    RestrictGroup0 = 0x00,
    RestrictGroup1 = 0x01,
    RestrictGroup2 = 0x10,
    None = 0x11,
}

enum SegmentationUpidType {
    NotUsed,
    UserDefinedDeprecated,
    ISCI,
    AdID,
    UMID,
    ISANDeprecated,
    ISAN,
    TID,
    TI,
    ADI,
    EIDR,
    ATSCContentIdentifier,
    MPU,
    MID,
    ADSInformation,
    URI,
    UUID,
    SCR,
    Reserved,
}

enum SegmentationUpid {
    NotUsed,
    UserDefinedDeprecated,
    ISCI,
    AdID,
    UMID,
    ISANDeprecated,
    ISAN,
    TID,
    TI,
    ADI,
    EIDR,
    ATSCContentIdentifier,
    MPU,
    MID,
    ADSInformation,
    URI,
    UUID,
    SCR,
    Reserved,
}

struct Component {
    component_tag: u8,
    pts_offset: u64,
}
