use deku::prelude::*;

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big", type="u8", bits = "1")]
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

    }
}

impl Default for SpliceTime {
    fn default() -> Self {
        Self::NoTimeSpecified {
            reserved: 0x7f
        }
    }
}

impl SpliceTime {
    fn new(pts_time: u64) -> Self {
        Self::TimeSpecified {
            reserved: 0x3f,
            pts_time
        }
    }
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
}