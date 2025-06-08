//! Core SCTE-35 data structures and types.
//!
//! This module contains the main structures representing SCTE-35 messages,
//! commands, and related components.

use crate::time::{DateTime, SpliceTime, BreakDuration};
use crate::descriptors::SpliceDescriptor;

/// Represents a complete SCTE-35 splice information section.
///
/// This is the top-level structure that contains all the information from an SCTE-35 message,
/// including the header fields, splice command, descriptors, and CRC.
///
/// # Fields
///
/// The structure follows the SCTE-35 specification layout:
/// - Header fields (table_id, section_length, etc.)
/// - Splice command data
/// - Optional descriptors
/// - CRC for data integrity
#[derive(Debug)]
pub struct SpliceInfoSection {
    /// Table identifier, should be 0xFC for SCTE-35
    pub table_id: u8,
    /// Section syntax indicator (0 for MPEG short section)
    pub section_syntax_indicator: u8,
    /// Private indicator (0 for not private)
    pub private_indicator: u8,
    /// SAP (Stream Access Point) type
    pub sap_type: u8,
    /// Length of the section in bytes
    pub section_length: u16,
    /// SCTE-35 protocol version
    pub protocol_version: u8,
    /// Encryption packet flag (0 for unencrypted)
    pub encrypted_packet: u8,
    /// Encryption algorithm used
    pub encryption_algorithm: u8,
    /// PTS adjustment value in 90kHz ticks
    pub pts_adjustment: u64,
    /// Control word index for encryption
    pub cw_index: u8,
    /// Tier value for authorization
    pub tier: u16,
    /// Length of the splice command in bytes
    pub splice_command_length: u16,
    /// Type of splice command (0x00-0xFF)
    pub splice_command_type: u8,
    /// The actual splice command data
    pub splice_command: SpliceCommand,
    /// Length of descriptor loop in bytes
    pub descriptor_loop_length: u16,
    /// List of splice descriptors
    pub splice_descriptors: Vec<SpliceDescriptor>,
    /// Alignment stuffing bits for byte alignment
    pub alignment_stuffing_bits: Vec<u8>,
    /// Encrypted CRC-32 (present when encrypted_packet = 1)
    pub e_crc_32: Option<u32>,
    /// CRC-32 checksum of the section
    pub crc_32: u32,
}

/// Represents the different types of splice commands defined in SCTE-35.
///
/// Each variant contains the specific data structure for that command type.
/// The command type determines how the splice operation should be performed.
#[derive(Debug)]
pub enum SpliceCommand {
    /// Null command (0x00) - No operation
    SpliceNull,
    /// Splice schedule command (0x04) - Scheduled splice events
    SpliceSchedule(SpliceSchedule),
    /// Splice insert command (0x05) - Ad insertion points
    SpliceInsert(SpliceInsert),
    /// Time signal command (0x06) - Time synchronization
    TimeSignal(TimeSignal),
    /// Bandwidth reservation command (0x07) - Bandwidth allocation
    BandwidthReservation(BandwidthReservation),
    /// Private command (0xFF) - Custom/proprietary commands
    PrivateCommand(PrivateCommand),
    /// Unknown command type
    Unknown,
}

/// Represents a splice null command.
///
/// This command indicates no splice operation should be performed.
/// It's used as a placeholder or to clear previous splice commands.
#[derive(Debug)]
pub struct SpliceNull {}

/// Represents a splice schedule command (0x04).
///
/// This command schedules splice events to occur at specific times in the future.
/// It allows for pre-scheduling of ad insertion points or other splice operations.
#[derive(Debug)]
pub struct SpliceSchedule {
    /// Unique identifier for this splice event
    pub splice_event_id: u32,
    /// Indicates if the splice event is being cancelled (1 = cancel, 0 = proceed)
    pub splice_event_cancel_indicator: u8,
    /// Reserved bits for future use
    pub reserved: u8,
    /// Indicates whether the splice is going out of or returning to the network (1 = out, 0 = in)
    pub out_of_network_indicator: u8,
    /// Indicates whether a duration is specified (1 = duration present, 0 = no duration)
    pub duration_flag: u8,
    /// Duration of the splice in 90kHz ticks (present when duration_flag = 1)
    pub splice_duration: Option<u32>,
    /// Scheduled time for the splice to occur (present when duration_flag = 0)
    pub scheduled_splice_time: Option<DateTime>,
    /// Unique identifier for the program
    pub unique_program_id: u16,
    /// Number of components in the component list
    pub num_splice: u8,
    /// List of component-specific splice information
    pub component_list: Vec<ComponentSplice>,
}

/// Represents a splice insert command (0x05).
///
/// This is the most commonly used splice command for ad insertion.
/// It signals the start and end of commercial breaks or other content substitutions.
#[derive(Debug)]
pub struct SpliceInsert {
    /// Unique identifier for this splice event
    pub splice_event_id: u32,
    /// Indicates if the splice event is being cancelled (1 = cancel, 0 = proceed)
    pub splice_event_cancel_indicator: u8,
    /// Reserved bits for future use
    pub reserved: u8,
    /// Indicates whether the splice is going out of or returning to the network (1 = out, 0 = in)
    pub out_of_network_indicator: u8,
    /// Indicates if this is a program-level splice (1) or component-level splice (0)
    pub program_splice_flag: u8,
    /// Indicates whether a break duration is specified (1 = duration present, 0 = no duration)
    pub duration_flag: u8,
    /// Indicates if the splice should happen immediately (1 = immediate, 0 = at specified time)
    pub splice_immediate_flag: u8,
    /// Additional reserved bits
    pub reserved2: u8,
    /// Presentation timestamp when the splice should occur (present when program_splice_flag = 1 and splice_immediate_flag = 0)
    pub splice_time: Option<SpliceTime>,
    /// Number of components in the component list (present when program_splice_flag = 0)
    pub component_count: u8,
    /// List of component-specific splice times (present when program_splice_flag = 0)
    pub components: Vec<SpliceInsertComponent>,
    /// Duration of the commercial break (present when duration_flag = 1)
    pub break_duration: Option<BreakDuration>,
    /// Unique identifier for the program
    pub unique_program_id: u16,
    /// Avail number for this splice event
    pub avail_num: u8,
    /// Expected number of avails in this break
    pub avails_expected: u8,
}

/// Represents a time signal command (0x06).
///
/// This command provides time synchronization information and is often used
/// with segmentation descriptors to indicate various types of content boundaries.
#[derive(Debug)]
pub struct TimeSignal {
    /// The presentation timestamp for this time signal
    pub splice_time: SpliceTime,
}

/// Represents a bandwidth reservation command (0x07).
///
/// This command is used to reserve bandwidth for future use,
/// typically in cable systems for managing network capacity.
#[derive(Debug)]
pub struct BandwidthReservation {
    /// Reserved bits for future use
    pub reserved: u8,
    /// Bandwidth reservation value in kilobits per second
    pub dwbw_reservation: u32,
}

/// Represents a private command (0xFF).
///
/// This command allows for custom, proprietary splice operations
/// that are not defined in the standard SCTE-35 specification.
#[derive(Debug)]
pub struct PrivateCommand {
    /// Identifier for the private command type
    pub private_command_id: u16,
    /// Length of the private command data in bytes
    pub private_command_length: u8,
    /// Raw bytes containing the private command data
    pub private_bytes: Vec<u8>,
}

/// Represents component-specific splice information for splice schedule commands.
///
/// This structure contains timing and mode information for individual components
/// when performing component-level splicing operations.
#[derive(Debug)]
pub struct ComponentSplice {
    /// Identifier for the specific component (audio/video track)
    pub component_tag: u8,
    /// Reserved bits for future use
    pub reserved: u8,
    /// Indicates the splice mode for this component
    pub splice_mode_indicator: u8,
    /// Indicates whether a duration is specified (1 = duration present, 0 = scheduled time present)
    pub duration_flag: u8,
    /// Duration of the splice for this component in 90kHz ticks (present when duration_flag = 1)
    pub splice_duration: Option<u32>,
    /// Scheduled time for the splice to occur (present when duration_flag = 0)
    pub scheduled_splice_time: Option<DateTime>,
}

/// Represents component-specific timing information for splice insert commands.
///
/// This structure contains the splice time for individual components
/// when performing component-level splice insert operations.
#[derive(Debug)]
pub struct SpliceInsertComponent {
    /// Identifier for the specific component (audio/video track)
    pub component_tag: u8,
    /// Presentation timestamp when this component should splice (present when splice_immediate_flag = 0)
    pub splice_time: Option<SpliceTime>,
}