use crate::{ClockTimeExt, CueError, TransportPacketWrite};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};
use std::io;
use std::time::Duration;

#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SpliceTime {
    time_specified_flag: bool,
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serialize_time"))]
    pts_time: u64,
}

impl SpliceTime {
    pub fn new() -> Self {
        Self {
            time_specified_flag: false,
            pts_time: 0,
        }
    }

    pub fn from_ticks(ticks: u64) -> Self {
        let mut splice_time = Self::new();
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
}

impl TransportPacketWrite for SpliceTime {
    fn write_to<W>(&self, buffer: &mut W) -> anyhow::Result<()>
    where
        W: io::Write,
    {
        let mut buffer = BitWriter::endian(buffer, BigEndian);
        buffer.write_bit(self.time_specified_flag)?;
        if self.time_specified_flag {
            buffer.write(6, 0x3f)?;
            buffer.write(33, self.pts_time)?;
        } else {
            buffer.write(7, 0x7f)?;
        }
        Ok(())
    }
}

impl From<Duration> for SpliceTime {
    fn from(duration: Duration) -> Self {
        Self::from_ticks(duration.to_90k())
    }
}
