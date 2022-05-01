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
    pub fn set_pts_time(&mut self, pts_time: Option<u64>) {
        match pts_time {
            None => {
                self.time_specified_flag = false;
                self.pts_time = 0;
            }
            Some(ticks) => {
                self.time_specified_flag = true;
                self.pts_time = ticks;
            }
        }
    }
}

impl TransportPacketWrite for SpliceTime {
    fn write_to<W>(&self, buffer: &mut W) -> Result<(), CueError>
    where
        W: io::Write,
    {
        let mut buffer = BitWriter::endian(buffer, BigEndian);
        buffer.write_bit(self.time_specified_flag)?;
        if self.time_specified_flag {
            buffer.write(6, 0x00)?;
            buffer.write(33, self.pts_time)?;
        } else {
            buffer.write(7, 0x00)?;
        }
        Ok(())
    }
}
