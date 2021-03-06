use crate::time::SpliceTime;
use crate::ClockTimeExt;
use std::io;
use std::io::Write;

#[cfg(feature = "serde")]
use serde::Serialize;

pub trait SpliceCommand {
    fn splice_command_type(&self) -> SpliceCommandType;

    fn write_to<W>(&mut self, buffer: &mut W) -> anyhow::Result<u32>
    where
        W: io::Write;
}

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SpliceNull {}

impl SpliceCommand for SpliceNull {
    fn splice_command_type(&self) -> SpliceCommandType {
        SpliceCommandType::SpliceNull
    }

    fn write_to<W>(&mut self, _: &mut W) -> anyhow::Result<u32>
    where
        W: io::Write,
    {
        Ok(0)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize), serde(transparent))]
#[repr(transparent)]
pub struct TimeSignal(SpliceTime);

impl TimeSignal {
    pub fn set_pts<T>(&mut self, pts: Option<T>)
    where
        T: ClockTimeExt,
    {
        self.0.set_pts_time(pts);
    }
}

impl SpliceCommand for TimeSignal {
    fn splice_command_type(&self) -> SpliceCommandType {
        SpliceCommandType::TimeSignal
    }

    fn write_to<W>(&mut self, buffer: &mut W) -> anyhow::Result<u32>
    where
        W: Write,
    {
        self.0.write_to(buffer)
    }
}

impl<T> From<T> for TimeSignal
where
    T: ClockTimeExt,
{
    fn from(pts: T) -> Self {
        let mut t = Self::default();
        t.set_pts(Some(pts));
        t
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
