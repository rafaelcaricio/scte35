use crate::time::SpliceTime;
use crate::{CueError, TransportPacketWrite};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use std::io;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

#[cfg(feature = "serde")]
use serde::Serialize;

pub trait SpliceCommand: TransportPacketWrite {
    fn splice_command_type(&self) -> SpliceCommandType;
}

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpliceNull {}

impl SpliceNull {
    pub fn new() -> SpliceNull {
        SpliceNull {}
    }
}

impl TransportPacketWrite for SpliceNull {
    fn write_to<W>(&self, _: &mut W) -> anyhow::Result<()>
    where
        W: io::Write,
    {
        Ok(())
    }
}

impl SpliceCommand for SpliceNull {
    fn splice_command_type(&self) -> SpliceCommandType {
        SpliceCommandType::SpliceNull
    }
}

#[cfg_attr(feature = "serde", derive(Serialize), serde(transparent))]
#[repr(transparent)]
pub struct TimeSignal(SpliceTime);

impl TimeSignal {
    pub fn new() -> Self {
        TimeSignal(SpliceTime::new())
    }

    pub fn from_ticks(pts_time: u64) -> Self {
        TimeSignal(SpliceTime::from_ticks(pts_time))
    }
}

impl TransportPacketWrite for TimeSignal {
    #[inline]
    fn write_to<W>(&self, buffer: &mut W) -> anyhow::Result<()>
    where
        W: Write,
    {
        self.0.write_to(buffer)
    }
}

impl SpliceCommand for TimeSignal {
    fn splice_command_type(&self) -> SpliceCommandType {
        SpliceCommandType::TimeSignal
    }
}

impl From<Duration> for TimeSignal {
    fn from(duration: Duration) -> Self {
        Self(duration.into())
    }
}

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpliceCommandType {
    SpliceNull,
    SpliceSchedule,
    SpliceInsert,
    TimeSignal,
    BandwidthReservation,
    PrivateCommand,
    Reserved(u8),
}

impl From<u8> for SpliceCommandType {
    fn from(value: u8) -> SpliceCommandType {
        match value {
            0x00 => SpliceCommandType::SpliceNull,
            0x04 => SpliceCommandType::SpliceSchedule,
            0x05 => SpliceCommandType::SpliceInsert,
            0x06 => SpliceCommandType::TimeSignal,
            0x07 => SpliceCommandType::BandwidthReservation,
            0xff => SpliceCommandType::PrivateCommand,
            _ => SpliceCommandType::Reserved(value),
        }
    }
}

impl From<SpliceCommandType> for u8 {
    fn from(value: SpliceCommandType) -> u8 {
        match value {
            SpliceCommandType::SpliceNull => 0x00,
            SpliceCommandType::SpliceSchedule => 0x04,
            SpliceCommandType::SpliceInsert => 0x05,
            SpliceCommandType::TimeSignal => 0x06,
            SpliceCommandType::BandwidthReservation => 0x07,
            SpliceCommandType::PrivateCommand => 0xff,
            SpliceCommandType::Reserved(value) => value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use assert_json_diff::assert_json_eq;

    #[cfg(feature = "serde")]
    #[test]
    fn serialize_splice_null() -> Result<()> {
        let splice_null = SpliceNull::new();
        assert_json_eq!(serde_json::to_value(&splice_null)?, serde_json::json!({}));
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize_time_signal() -> Result<()> {
        let time_signal = TimeSignal::new();
        assert_json_eq!(
            serde_json::to_value(&time_signal)?,
            serde_json::json!({
                "time_specified_flag": false,
                "pts_time": 0.0
            })
        );
        Ok(())
    }
}
