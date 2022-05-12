use ascii::AsciiString;
use deku::prelude::*;
use crate::{DeviceRestrictions};

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big", type = "u8", bits = "1")]
pub(crate) enum SpliceTime {
    #[deku(id = "1")]
    TimeSpecified {
        #[deku(bits = "6", assert_eq = "0x3f", update = "0x3f")]
        _reserved: u8,

        #[deku(bits = "33")]
        pts_time: u64,
    },

    #[deku(id = "0")]
    NoTimeSpecified {
        #[deku(bits = "7", assert_eq = "0x7f", update = "0x7f")]
        _reserved: u8,
    },
}

impl Default for SpliceTime {
    fn default() -> Self {
        Self::NoTimeSpecified { _reserved: 0x7f }
    }
}

impl SpliceTime {
    fn new(pts_time: u64) -> Self {
        Self::TimeSpecified {
            _reserved: 0x3f,
            pts_time,
        }
    }
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
pub(crate) enum SpliceDescriptor {
    #[deku(id = "0x02")]
    SegmentationDescriptor(SegmentationDescriptor),

    #[deku(id_pat = "_")]
    Template(GenericDescriptor),
}

impl SpliceDescriptor {
    pub(crate) fn update(&mut self) -> Result<(), deku::DekuError> {
        use SpliceDescriptor::*;
        match self {
            Template(s) => s.update(),
            SegmentationDescriptor(s) => s.update(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub(crate) struct GenericDescriptor {
    id: u8,

    #[deku(update = "self.private_bytes.len() + 2")]
    descriptor_length: u8,

    identifier: u32,

    #[deku(count = "descriptor_length - 2")]
    private_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
pub(crate) struct SegmentationDescriptor {
    descriptor_length: u8, // TODO: need to calculate by hand the size based in `self.*`

    identifier: u32,

    segmentation_event_id: u32,

    #[deku(bits = "1")]
    segmentation_event_cancel_indicator: bool,

    #[deku(bits = "7", assert_eq = "0x7f", update = "0x7f")]
    _reserved: u8,

    #[deku(
        skip,
        cond = "*segmentation_event_cancel_indicator == false",
        default = "None"
    )]
    segmentation: Option<Segmentation>,
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
pub(crate) struct Segmentation {
    #[deku(bits = "1")]
    program_segmentation_flag: bool,

    #[deku(bits = "1")]
    segmentation_duration_flag: bool,

    #[deku(bits = "1")]
    delivery_not_restricted_flag: bool,

    #[deku(cond = "*delivery_not_restricted_flag == false")]
    delivery_restriction: Option<DeliveryRestriction>,

    #[deku(cond = "*delivery_not_restricted_flag", bits = "5")]
    _reserved1: Option<u8>,

    #[deku(cond = "*program_segmentation_flag == false")]
    program_components: Option<ProgramComponents>,

    #[deku(cond = "*segmentation_duration_flag", bits = "40")]
    segmentation_duration: u64,

    segmentation_upid: SegmentationUpid,
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
#[non_exhaustive]
pub enum SegmentationUpid {
    #[deku(id = "0x00")]
    NotUsed,

    #[deku(id = "0x01")]
    UserDefinedDeprecated(AsciiString),

    #[deku(id = "0x02")]
    ISCI(AsciiString),

    #[deku(id = "0x03")]
    AdID(AsciiString),

    #[deku(id = "0x04")]
    UMID(AsciiString),

    #[deku(id = "0x05")]
    ISANDeprecated(u64),

    #[deku(id = "0x06")]
    ISAN(u128),

    #[deku(id = "0x07")]
    TID(AsciiString),

    #[deku(id = "0x08")]
    AiringID(u64),

    #[deku(id = "0x09")]
    ADI(AsciiString),

    #[deku(id = "0x0a")]
    EIDR(u128),

    #[deku(id = "0x0b")]
    ATSCContentIdentifier(AsciiString),

    #[deku(id = "0x0c")]
    MPU,

    #[deku(id = "0x0d")]
    MID,

    #[deku(id = "0x0e")]
    ADSInformation(AsciiString),

    #[deku(id = "0x0f")]
    URI(AsciiString),

    #[deku(id = "0x10")]
    UUID(u128),

    #[deku(id = "0x11")]
    SCR(AsciiString),

    #[deku(id_pat = "_")]
    Reserved(u8),
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
pub(crate) struct ProgramComponents {
    #[deku(update = "self.components.len()")]
    component_count: u8,

    #[deku(count = "component_count")]
    components: Vec<SegmentationDescriptorComponent>
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
pub(crate) struct SegmentationDescriptorComponent {
    component_tag: u8,

    #[deku(bits = "7")]
    _reserved: u8,

    #[deku(bits = "33")]
    pts_offset: u64,
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
pub(crate) struct DeliveryRestriction {
    #[deku(bits = "1")]
    web_delivery_allowed_flag: bool,

    #[deku(bits = "1")]
    no_regional_blackout_flag: bool,

    #[deku(bits = "1")]
    archive_allowed_flag: bool,

    device_restrictions: DeviceRestrictions,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_splice_time() {
        let st = SpliceTime::default();
        let data: Vec<u8> = st.try_into().unwrap();

        assert_eq!(hex::encode(data.as_slice()), "7f");

        // Check update is defined for missing value in field
        let mut st = SpliceTime::NoTimeSpecified {
            _reserved: 0
        };
        st.update().unwrap();

        let data: Vec<u8> = st.try_into().unwrap();

        assert_eq!(hex::encode(data.as_slice()), "7f");
    }

    #[test]
    fn write_splice_time_with_pts_time() {
        let st = SpliceTime::new(0x072bd0050);
        let encoded: Vec<u8> = st.try_into().unwrap();

        assert_eq!(hex::encode(encoded.as_slice()), "fe72bd0050");
    }

    #[test]
    fn read_splice_time_with_pts_time() {
        let data = hex::decode("fe72bd0050").unwrap();
        let st = SpliceTime::try_from(data.as_slice()).unwrap();

        assert_eq!(st, SpliceTime::new(0x072bd0050));
    }

    #[test]
    fn write_generic_descriptor() {
        let mut sd = SpliceDescriptor::Template(GenericDescriptor {
            id: 0xff,
            descriptor_length: 0,
            identifier: 0x43554549,
            private_bytes: vec![0x01],
        });

        sd.update().unwrap();

        let data: Vec<u8> = sd.try_into().unwrap();

        assert_eq!(hex::encode(data.as_slice()), "ff034355454901");
    }

    #[test]
    fn read_generic_descriptor() {
        let data = hex::decode("ff034355454901").unwrap();
        let sd = SpliceDescriptor::try_from(data.as_slice()).unwrap();

        assert_eq!(
            sd,
            SpliceDescriptor::Template(GenericDescriptor {
                id: 0xff,
                descriptor_length: 3,
                identifier: 0x43554549,
                private_bytes: vec![0x01]
            })
        );
    }
}
