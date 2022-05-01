use crate::{CueError, TransportPacketWrite};
use std::io;

#[cfg(feature = "serde")]
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SegmentationDescriptor {
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
        W: io::Write,
    {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
struct DeliveryRestriction {
    web_delivery_allowed_flag: bool,
    no_regional_blackout_flag: bool,
    archive_allowed_flag: bool,
    device_restrictions: DeviceRestrictions,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
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

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
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

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
struct Component {
    component_tag: u8,
    pts_offset: u64,
}
