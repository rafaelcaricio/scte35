//! Encoding implementations for SCTE-35 splice commands.

use crate::encoding::{BitWriter, Encodable, EncodingResult};
use crate::types::*;

impl Encodable for SpliceCommand {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        match self {
            SpliceCommand::SpliceNull => {
                // SpliceNull has no payload
                Ok(())
            }
            SpliceCommand::SpliceSchedule(schedule) => schedule.encode(writer),
            SpliceCommand::SpliceInsert(insert) => insert.encode(writer),
            SpliceCommand::TimeSignal(signal) => signal.encode(writer),
            SpliceCommand::BandwidthReservation(reservation) => reservation.encode(writer),
            SpliceCommand::PrivateCommand(private) => private.encode(writer),
            SpliceCommand::Unknown => {
                // Unknown command has no defined encoding
                Ok(())
            }
        }
    }

    fn encoded_size(&self) -> usize {
        match self {
            SpliceCommand::SpliceNull => 0,
            SpliceCommand::SpliceSchedule(schedule) => schedule.encoded_size(),
            SpliceCommand::SpliceInsert(insert) => insert.encoded_size(),
            SpliceCommand::TimeSignal(signal) => signal.encoded_size(),
            SpliceCommand::BandwidthReservation(reservation) => reservation.encoded_size(),
            SpliceCommand::PrivateCommand(private) => private.encoded_size(),
            SpliceCommand::Unknown => 0,
        }
    }
}

impl Encodable for SpliceInsert {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // splice_event_id (32 bits)
        writer.write_bits(self.splice_event_id as u64, 32)?;

        // splice_event_cancel_indicator (1 bit)
        writer.write_bits(self.splice_event_cancel_indicator as u64, 1)?;

        // reserved (7 bits)
        writer.write_bits(self.reserved as u64, 7)?;

        if self.splice_event_cancel_indicator == 0 {
            // out_of_network_indicator (1 bit)
            writer.write_bits(self.out_of_network_indicator as u64, 1)?;

            // program_splice_flag (1 bit)
            writer.write_bits(self.program_splice_flag as u64, 1)?;

            // duration_flag (1 bit)
            writer.write_bits(self.duration_flag as u64, 1)?;

            // splice_immediate_flag (1 bit)
            writer.write_bits(self.splice_immediate_flag as u64, 1)?;

            // event_id_compliance_flag (1 bit) - assuming reserved2 contains this
            writer.write_bits((self.reserved2 >> 2) as u64 & 1, 1)?;

            // reserved (3 bits)
            writer.write_bits(self.reserved2 as u64 & 0x7, 3)?;

            // Encode splice_time if program_splice_flag == 1 and splice_immediate_flag == 0
            if self.program_splice_flag == 1 && self.splice_immediate_flag == 0 {
                if let Some(ref splice_time) = self.splice_time {
                    splice_time.encode(writer)?;
                }
            }

            // Encode component-specific splice times if program_splice_flag == 0
            if self.program_splice_flag == 0 {
                // component_count (8 bits)
                writer.write_bits(self.component_count as u64, 8)?;

                for component in &self.components {
                    component.encode(writer)?;
                }
            }
        }

        // Encode break_duration if duration_flag == 1
        if self.duration_flag == 1 {
            if let Some(ref break_duration) = self.break_duration {
                break_duration.encode(writer)?;
            }
        }

        // unique_program_id (16 bits)
        writer.write_bits(self.unique_program_id as u64, 16)?;

        // avail_num (8 bits)
        writer.write_bits(self.avail_num as u64, 8)?;

        // avails_expected (8 bits)
        writer.write_bits(self.avails_expected as u64, 8)?;

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        let mut size = 4; // splice_event_id
        size += 1; // splice_event_cancel_indicator + reserved

        if self.splice_event_cancel_indicator == 0 {
            size += 1; // flags byte

            // splice_time
            if self.program_splice_flag == 1 && self.splice_immediate_flag == 0 {
                if let Some(ref splice_time) = self.splice_time {
                    size += splice_time.encoded_size();
                }
            }

            // components
            if self.program_splice_flag == 0 {
                size += 1; // component_count
                for component in &self.components {
                    size += component.encoded_size();
                }
            }
        }

        // break_duration
        if self.duration_flag == 1 {
            if let Some(ref break_duration) = self.break_duration {
                size += break_duration.encoded_size();
            }
        }

        size += 4; // unique_program_id + avail_num + avails_expected

        size
    }
}

impl Encodable for TimeSignal {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // Time signal contains a splice_time
        self.splice_time.encode(writer)?;
        Ok(())
    }

    fn encoded_size(&self) -> usize {
        self.splice_time.encoded_size()
    }
}

impl Encodable for SpliceSchedule {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // splice_count (8 bits) - number of splice events
        writer.write_bits(1u64, 8)?; // For simplicity, encoding as single event

        // splice_event_id (32 bits)
        writer.write_bits(self.splice_event_id as u64, 32)?;

        // splice_event_cancel_indicator (1 bit)
        writer.write_bits(self.splice_event_cancel_indicator as u64, 1)?;

        // event_id_compliance_flag (1 bit) - part of reserved
        writer.write_bits((self.reserved >> 6) as u64 & 1, 1)?;

        // reserved (6 bits)
        writer.write_bits(self.reserved as u64 & 0x3F, 6)?;

        if self.splice_event_cancel_indicator == 0 {
            // out_of_network_indicator (1 bit)
            writer.write_bits(self.out_of_network_indicator as u64, 1)?;

            // program_splice_flag (1 bit) - assuming always 1 for program-level
            writer.write_bits(1u64, 1)?;

            // duration_flag (1 bit)
            writer.write_bits(self.duration_flag as u64, 1)?;

            // reserved (5 bits) - should be all 1s
            writer.write_bits(0x1F, 5)?; // 0x1F = 11111 in binary

            // utc_splice_time (32 bits) if program_splice_flag == 1
            if let Some(utc_time) = self.utc_splice_time {
                writer.write_bits(utc_time as u64, 32)?;
            }

            // break_duration if duration_flag == 1
            if self.duration_flag == 1 {
                if let Some(splice_duration) = self.splice_duration {
                    writer.write_bits(splice_duration as u64, 40)?; // break_duration is 40 bits
                }
            }

            // unique_program_id (16 bits)
            writer.write_bits(self.unique_program_id as u64, 16)?;

            // avail_num (8 bits)
            writer.write_bits(self.num_splice as u64, 8)?;

            // avails_expected (8 bits) - using num_splice as placeholder
            writer.write_bits(self.num_splice as u64, 8)?;
        }

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        let mut size = 1 + 4 + 1; // splice_count + splice_event_id + flags

        if self.splice_event_cancel_indicator == 0 {
            size += 1; // flags
            size += 4; // utc_splice_time

            if self.duration_flag == 1 {
                size += 5; // break_duration (40 bits = 5 bytes)
            }

            size += 4; // unique_program_id + avail_num + avails_expected
        }

        size
    }
}

impl Encodable for BandwidthReservation {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // reserved (8 bits)
        writer.write_bits(self.reserved as u64, 8)?;

        // dwbw_reservation (32 bits)
        writer.write_bits(self.dwbw_reservation as u64, 32)?;

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        5 // 8 + 32 bits = 40 bits = 5 bytes
    }
}

impl Encodable for PrivateCommand {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // private_command_id (16 bits)
        writer.write_bits(self.private_command_id as u64, 16)?;

        // private_command_length (8 bits)
        writer.write_bits(self.private_command_length as u64, 8)?;

        // private_bytes
        writer.write_bytes(&self.private_bytes)?;

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        2 + 1 + self.private_bytes.len() // id + length + data
    }
}

impl Encodable for SpliceInsertComponent {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // component_tag (8 bits)
        writer.write_bits(self.component_tag as u64, 8)?;

        // splice_time if not immediate
        if let Some(ref splice_time) = self.splice_time {
            splice_time.encode(writer)?;
        }

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        let mut size = 1; // component_tag

        if let Some(ref splice_time) = self.splice_time {
            size += splice_time.encoded_size();
        }

        size
    }
}
