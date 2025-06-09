//! Extensions for existing types to support the builder pattern.

use crate::types::SpliceCommand;

/// Extension trait to provide encoding length calculation for SpliceCommand.
pub trait SpliceCommandExt {
    /// Calculate the encoded length of this splice command in bytes.
    fn encoded_length(&self) -> u16;
}

impl SpliceCommandExt for SpliceCommand {
    fn encoded_length(&self) -> u16 {
        match self {
            SpliceCommand::SpliceNull => 0,
            SpliceCommand::SpliceInsert(insert) => {
                // Base: 14 bytes for fixed fields
                let mut len = 14;
                
                // Add splice_time if present (5 bytes)
                if insert.program_splice_flag == 1 && insert.splice_immediate_flag == 0 {
                    len += 5;
                }
                
                // Add component data if present
                if insert.program_splice_flag == 0 {
                    len += 1; // component_count
                    len += insert.components.len() * 6; // each component: 1 + 5 bytes
                }
                
                // Add break_duration if present (5 bytes)
                if insert.duration_flag == 1 {
                    len += 5;
                }
                
                len as u16
            }
            SpliceCommand::TimeSignal(_) => 5, // splice_time only
            SpliceCommand::BandwidthReservation(_) => 4, // Fixed 4 bytes
            SpliceCommand::SpliceSchedule(schedule) => {
                // Base: 5 bytes (splice_event_id + flags)
                let mut len = 5;
                
                // Add scheduled_splice_time or splice_duration
                if schedule.duration_flag == 1 {
                    len += 4; // splice_duration
                } else if schedule.scheduled_splice_time.is_some() {
                    len += 9; // DateTime structure
                }
                
                // Add component list
                len += 2; // unique_program_id
                len += 1; // num_splice
                len += schedule.component_list.len() * 8; // estimated component size
                
                len as u16
            }
            SpliceCommand::PrivateCommand(pc) => {
                // identifier (4 bytes) + private_bytes
                4 + pc.private_bytes.len() as u16
            }
            SpliceCommand::Unknown => 0,
        }
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