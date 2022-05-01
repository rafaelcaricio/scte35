use crate::{CueError, TransportPacketWrite};
use std::io;

#[cfg(feature = "serde")]
use serde::Serialize;

pub trait SpliceCommand: TransportPacketWrite {
    fn splice_command_type(&self) -> SpliceCommandType;
}

#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SpliceNull {}

impl SpliceNull {
    pub fn new() -> SpliceNull {
        SpliceNull {}
    }
}

impl TransportPacketWrite for SpliceNull {
    fn write_to<W>(&self, _: &mut W) -> Result<(), CueError>
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

#[derive(Debug, Clone, Copy)]
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
