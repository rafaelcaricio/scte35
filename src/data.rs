use deku::prelude::*;

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big", type = "u8", bits = "1")]
pub(crate) enum SpliceTime {
    #[deku(id = "1")]
    TimeSpecified {
        #[deku(bits = "6", assert_eq = "0x3f")]
        reserved: u8,
        #[deku(bits = "33")]
        pts_time: u64,
    },
    #[deku(id = "0")]
    NoTimeSpecified {
        #[deku(bits = "7", assert_eq = "0x7f")]
        reserved: u8,
    },
}

impl Default for SpliceTime {
    fn default() -> Self {
        Self::NoTimeSpecified { reserved: 0x7f }
    }
}

impl SpliceTime {
    fn new(pts_time: u64) -> Self {
        Self::TimeSpecified {
            reserved: 0x3f,
            pts_time,
        }
    }
}

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u8")]
pub(crate) enum SpliceDescriptor {
    // #[deku(id = "0x02")]
    // SegmentationDescriptor {
    //     #[deku(update = "deku::rest")]
    //     descriptor_length: u8,
    //     identifier: u32,
    //
    // },
    #[deku(id_pat = "_")]
    Template(GenericDescriptor),
}

impl SpliceDescriptor {
    pub(crate) fn update(&mut self) -> Result<(), deku::DekuError> {
        match self {
            SpliceDescriptor::Template(s) => s.update(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_splice_time() {
        let st = SpliceTime::default();
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
