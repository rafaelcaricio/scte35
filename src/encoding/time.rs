//! Encoding implementations for SCTE-35 time structures.

use crate::encoding::{BitWriter, Encodable, EncodingResult};
use crate::time::*;

impl Encodable for SpliceTime {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // time_specified_flag (1 bit)
        writer.write_bits(self.time_specified_flag as u64, 1)?;

        if self.time_specified_flag != 0 {
            // reserved (6 bits) - should be all 1s
            writer.write_bits(0x3F, 6)?; // 0x3F = 111111 in binary

            // pts_time (33 bits)
            if let Some(pts_time) = self.pts_time {
                writer.write_bits(pts_time & 0x1FFFFFFFF, 33)?;
            } else {
                writer.write_bits(0u64, 33)?;
            }
        } else {
            // reserved (7 bits) - should be all 1s
            writer.write_bits(0x7F, 7)?; // 0x7F = 1111111 in binary
        }

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        if self.time_specified_flag != 0 {
            5 // 1 + 6 + 33 bits = 40 bits = 5 bytes
        } else {
            1 // 1 + 7 bits = 8 bits = 1 byte
        }
    }
}

impl Encodable for BreakDuration {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // auto_return (1 bit)
        writer.write_bits(self.auto_return as u64, 1)?;

        // reserved (6 bits) - should be all 1s
        writer.write_bits(0x3F, 6)?; // 0x3F = 111111 in binary

        // duration (33 bits)
        writer.write_bits(self.duration & 0x1FFFFFFFF, 33)?;

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        5 // 1 + 6 + 33 bits = 40 bits = 5 bytes
    }
}
