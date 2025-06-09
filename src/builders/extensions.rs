//! Extensions for existing types to support the builder pattern.

use crate::encoding::Encodable;
use crate::types::SpliceCommand;

/// Extension trait to provide encoding length calculation for SpliceCommand.
pub trait SpliceCommandExt {
    /// Calculate the encoded length of this splice command in bytes.
    fn encoded_length(&self) -> u16;
}

impl SpliceCommandExt for SpliceCommand {
    fn encoded_length(&self) -> u16 {
        // Use the actual Encodable trait implementation for accurate sizing
        self.encoded_size() as u16
    }
}

/// Convert SpliceCommand reference to command type byte.
impl From<&SpliceCommand> for u8 {
    fn from(command: &SpliceCommand) -> Self {
        match command {
            SpliceCommand::SpliceNull => 0x00,
            SpliceCommand::SpliceSchedule(_) => 0x04,
            SpliceCommand::SpliceInsert(_) => 0x05,
            SpliceCommand::TimeSignal(_) => 0x06,
            SpliceCommand::BandwidthReservation(_) => 0x07,
            SpliceCommand::PrivateCommand(_) => 0xFF,
            SpliceCommand::Unknown => 0xFF,
        }
    }
}
