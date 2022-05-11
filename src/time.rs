use crate::{BytesWritten, ClockTimeExt};
use bitstream_io::{BigEndian, BitRecorder, BitWrite, BitWriter};
use std::io;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpliceTime {
    time_specified_flag: bool,
    pts_time: u64,

    // Size of the SpliceTime structure after encoding
    pub(crate) bytes_length: Option<u32>,
}

impl SpliceTime {
    pub fn from_ticks(ticks: u64) -> Self {
        let mut splice_time = Self::default();
        splice_time.set_pts_time(Some(ticks));
        splice_time
    }

    #[inline]
    pub fn set_pts_time<T>(&mut self, pts_time: Option<T>)
    where
        T: ClockTimeExt,
    {
        match pts_time {
            None => {
                self.time_specified_flag = false;
                self.pts_time = 0;
            }
            Some(duration) => {
                self.time_specified_flag = true;
                self.pts_time = duration.to_90k();
            }
        }
    }

    pub fn time_specified_flag(&self) -> bool {
        self.time_specified_flag
    }

    pub fn pts_time(&self) -> Option<u64> {
        if self.time_specified_flag {
            Some(self.pts_time)
        } else {
            None
        }
    }

    pub(crate) fn write_to<W>(&mut self, buffer: &mut W) -> anyhow::Result<u32>
    where
        W: io::Write,
    {
        let mut recorder = BitRecorder::<u32, BigEndian>::new();

        recorder.write_bit(self.time_specified_flag)?;
        if self.time_specified_flag {
            recorder.write(6, 0x3f)?;
            recorder.write(33, self.pts_time)?;
        } else {
            recorder.write(7, 0x7f)?;
        }

        let mut buffer = BitWriter::endian(buffer, BigEndian);
        recorder.playback(&mut buffer)?;

        self.bytes_length = Some(recorder.bytes_written());

        Ok(recorder.bytes_written())
    }
}

impl<T> From<T> for SpliceTime
where
    T: ClockTimeExt,
{
    fn from(pts: T) -> Self {
        let mut t = Self::default();
        t.set_pts_time(Some(pts));
        t
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encode_time_signal() {
        let mut st = SpliceTime::default();

        let mut data = Vec::new();
        st.write_to(&mut data).unwrap();

        assert_eq!(hex::encode(data.as_slice()), "7f")
    }

    #[test]
    fn encode_time_signal_with_time() {
        let mut st = SpliceTime::from(0x072bd0050);

        let mut data = Vec::new();
        st.write_to(&mut data).unwrap();

        assert_eq!(hex::encode(data.as_slice()), "fe72bd0050")
    }
}
