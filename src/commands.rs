//! SCTE-35 splice command parsing.
//!
//! This module contains functions for parsing different types of splice commands
//! from binary data.

use crate::bit_reader::BitReader;
use crate::time::{BreakDuration, DateTime, SpliceTime};
use crate::types::{
    BandwidthReservation, ComponentSplice, PrivateCommand, SpliceCommand, SpliceInsert,
    SpliceInsertComponent, SpliceSchedule, TimeSignal,
};
use std::io;

/// Parses a splice command based on the command type.
///
/// This function dispatches to the appropriate parsing function based on the
/// command type identifier.
pub(crate) fn parse_splice_command(
    reader: &mut BitReader,
    splice_command_type: u8,
    splice_command_length: u16,
) -> Result<SpliceCommand, io::Error> {
    match splice_command_type {
        0x00 => Ok(SpliceCommand::SpliceNull),
        0x04 => Ok(SpliceCommand::SpliceSchedule(parse_splice_schedule(
            reader,
        )?)),
        0x05 => Ok(SpliceCommand::SpliceInsert(parse_splice_insert(reader)?)),
        0x06 => Ok(SpliceCommand::TimeSignal(parse_time_signal(reader)?)),
        0x07 => Ok(SpliceCommand::BandwidthReservation(
            parse_bandwidth_reservation(reader)?,
        )),
        0xFF => Ok(SpliceCommand::PrivateCommand(parse_private_command(
            reader,
        )?)),
        _ => {
            // Unknown command type - skip the data
            reader.skip_bits((splice_command_length * 8) as usize)?;
            Ok(SpliceCommand::Unknown)
        }
    }
}

/// Parses a splice schedule command (0x04).
pub(crate) fn parse_splice_schedule(reader: &mut BitReader) -> Result<SpliceSchedule, io::Error> {
    let splice_event_id = reader.read_uimsbf(32)? as u32;
    let splice_event_cancel_indicator = reader.read_bslbf(1)? as u8;
    let reserved = reader.read_bslbf(7)? as u8;
    let out_of_network_indicator = reader.read_bslbf(1)? as u8;
    let duration_flag = reader.read_bslbf(1)? as u8;

    let splice_duration = if duration_flag == 1 {
        Some(reader.read_uimsbf(32)? as u32)
    } else {
        None
    };

    let scheduled_splice_time = if duration_flag == 0 {
        let _reserved = reader.read_bslbf(5)? as u8;
        Some(parse_date_time(reader)?)
    } else {
        None
    };

    let unique_program_id = reader.read_uimsbf(16)? as u16;
    let num_splice = reader.read_uimsbf(8)? as u8;
    let mut component_list = Vec::new();
    for _ in 0..num_splice {
        component_list.push(parse_component_splice(reader)?);
    }

    Ok(SpliceSchedule {
        splice_event_id,
        splice_event_cancel_indicator,
        reserved,
        out_of_network_indicator,
        duration_flag,
        splice_duration,
        scheduled_splice_time,
        unique_program_id,
        num_splice,
        component_list,
    })
}

/// Parses a splice insert command (0x05).
pub(crate) fn parse_splice_insert(reader: &mut BitReader) -> Result<SpliceInsert, io::Error> {
    let splice_event_id = reader.read_uimsbf(32)? as u32;
    let splice_event_cancel_indicator = reader.read_bslbf(1)? as u8;
    let reserved = reader.read_bslbf(7)? as u8;

    if splice_event_cancel_indicator == 1 {
        // If cancel indicator is set, no other fields follow
        return Ok(SpliceInsert {
            splice_event_id,
            splice_event_cancel_indicator,
            reserved,
            out_of_network_indicator: 0,
            program_splice_flag: 0,
            duration_flag: 0,
            splice_immediate_flag: 0,
            reserved2: 0,
            splice_time: None,
            component_count: 0,
            components: Vec::new(),
            break_duration: None,
            unique_program_id: 0,
            avail_num: 0,
            avails_expected: 0,
        });
    }

    let out_of_network_indicator = reader.read_bslbf(1)? as u8;
    let program_splice_flag = reader.read_bslbf(1)? as u8;
    let duration_flag = reader.read_bslbf(1)? as u8;
    let splice_immediate_flag = reader.read_bslbf(1)? as u8;
    let reserved2 = reader.read_bslbf(4)? as u8;

    let splice_time = if program_splice_flag == 1 && splice_immediate_flag == 0 {
        Some(parse_splice_time(reader)?)
    } else {
        None
    };

    let component_count = if program_splice_flag == 0 {
        reader.read_uimsbf(8)? as u8
    } else {
        0
    };

    let mut components = Vec::new();
    if program_splice_flag == 0 {
        for _ in 0..component_count {
            let component_tag = reader.read_uimsbf(8)? as u8;
            let splice_time = if splice_immediate_flag == 0 {
                Some(parse_splice_time(reader)?)
            } else {
                None
            };
            components.push(SpliceInsertComponent {
                component_tag,
                splice_time,
            });
        }
    }

    let break_duration = if duration_flag == 1 {
        Some(parse_break_duration(reader)?)
    } else {
        None
    };

    let unique_program_id = reader.read_uimsbf(16)? as u16;
    let avail_num = reader.read_uimsbf(8)? as u8;
    let avails_expected = reader.read_uimsbf(8)? as u8;

    Ok(SpliceInsert {
        splice_event_id,
        splice_event_cancel_indicator,
        reserved,
        out_of_network_indicator,
        program_splice_flag,
        duration_flag,
        splice_immediate_flag,
        reserved2,
        splice_time,
        component_count,
        components,
        break_duration,
        unique_program_id,
        avail_num,
        avails_expected,
    })
}

/// Parses a time signal command (0x06).
pub(crate) fn parse_time_signal(reader: &mut BitReader) -> Result<TimeSignal, io::Error> {
    let splice_time = parse_splice_time(reader)?;
    Ok(TimeSignal { splice_time })
}

/// Parses a bandwidth reservation command (0x07).
pub(crate) fn parse_bandwidth_reservation(
    reader: &mut BitReader,
) -> Result<BandwidthReservation, io::Error> {
    let reserved = reader.read_bslbf(8)? as u8;
    let dwbw_reservation = reader.read_uimsbf(32)? as u32;
    Ok(BandwidthReservation {
        reserved,
        dwbw_reservation,
    })
}

/// Parses a private command (0xFF).
pub(crate) fn parse_private_command(reader: &mut BitReader) -> Result<PrivateCommand, io::Error> {
    let private_command_id = reader.read_uimsbf(16)? as u16;
    let private_command_length = reader.read_uimsbf(8)? as u8;
    let mut private_bytes = Vec::new();
    for _ in 0..private_command_length {
        private_bytes.push(reader.read_uimsbf(8)? as u8);
    }
    Ok(PrivateCommand {
        private_command_id,
        private_command_length,
        private_bytes,
    })
}

/// Parses a splice time structure.
pub(crate) fn parse_splice_time(reader: &mut BitReader) -> Result<SpliceTime, io::Error> {
    let time_specified_flag = reader.read_bslbf(1)? as u8;
    let pts_time = if time_specified_flag == 1 {
        let _reserved = reader.read_bslbf(6)? as u8;
        Some(reader.read_uimsbf(33)?)
    } else {
        let _reserved = reader.read_bslbf(7)? as u8;
        None
    };
    Ok(SpliceTime {
        time_specified_flag,
        pts_time,
    })
}

/// Parses a break duration structure.
pub(crate) fn parse_break_duration(reader: &mut BitReader) -> Result<BreakDuration, io::Error> {
    let auto_return = reader.read_bslbf(1)? as u8;
    let reserved = reader.read_bslbf(6)? as u8;
    let duration = reader.read_uimsbf(33)?;
    Ok(BreakDuration {
        auto_return,
        reserved,
        duration,
    })
}

/// Parses a date/time structure.
pub(crate) fn parse_date_time(reader: &mut BitReader) -> Result<DateTime, io::Error> {
    let utc_flag = reader.read_bslbf(1)? as u8;
    let year = reader.read_uimsbf(12)? as u16;
    let month = reader.read_uimsbf(4)? as u8;
    let day = reader.read_uimsbf(5)? as u8;
    let hour = reader.read_uimsbf(5)? as u8;
    let minute = reader.read_uimsbf(6)? as u8;
    let second = reader.read_uimsbf(6)? as u8;
    let frames = reader.read_uimsbf(6)? as u8;
    let milliseconds = reader.read_uimsbf(3)? as u8;
    Ok(DateTime {
        utc_flag,
        year,
        month,
        day,
        hour,
        minute,
        second,
        frames,
        milliseconds,
    })
}

/// Parses a component splice structure.
pub(crate) fn parse_component_splice(reader: &mut BitReader) -> Result<ComponentSplice, io::Error> {
    let component_tag = reader.read_uimsbf(8)? as u8;
    let reserved = reader.read_bslbf(5)? as u8;
    let splice_mode_indicator = reader.read_bslbf(1)? as u8;
    let duration_flag = reader.read_bslbf(1)? as u8;

    let splice_duration = if duration_flag == 1 {
        Some(reader.read_uimsbf(32)? as u32)
    } else {
        None
    };

    let scheduled_splice_time = if duration_flag == 0 {
        let _reserved = reader.read_bslbf(5)? as u8;
        Some(parse_date_time(reader)?)
    } else {
        None
    };

    Ok(ComponentSplice {
        component_tag,
        reserved,
        splice_mode_indicator,
        duration_flag,
        splice_duration,
        scheduled_splice_time,
    })
}
