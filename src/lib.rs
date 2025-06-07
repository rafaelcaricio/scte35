//! # SCTE-35 Parsing Library
//!
//! This library provides functionality to parse SCTE-35 (Society of Cable 
//! Telecommunications Engineers) messages, which are used for inserting cue 
//! messages into video streams for ad insertion points in broadcast television.
//!
//! ## Features
//!
//! - Zero-dependency library (optional CLI tool requires base64)
//! - Bit-level parsing of SCTE-35 binary messages
//! - Support for all major splice command types
//! - Time conversion utilities (90kHz ticks to std::time::Duration)
//! - String conversion for descriptor data
//!
//! ## Example
//!
//! ```rust
//! use scte35_parsing::parse_splice_info_section;
//! use base64::{Engine, engine::general_purpose};
//!
//! let base64_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
//! let buffer = general_purpose::STANDARD.decode(base64_message).unwrap();
//! let section = parse_splice_info_section(&buffer).unwrap();
//!
//! println!("Table ID: 0x{:02X}", section.table_id);
//! println!("Command Type: 0x{:02X}", section.splice_command_type);
//! ```

#![warn(missing_docs)]

use std::io::{self, ErrorKind};
use std::time::Duration;

// CRC validation module - only included when feature is enabled
#[cfg(feature = "crc-validation")]
pub mod crc;

// Re-export commonly used CRC functions for convenience - only when available
#[cfg(feature = "crc-validation")]
pub use crc::{validate_message_crc, CrcValidatable};

// Helper struct to read bits from a byte slice
struct BitReader<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> BitReader<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        BitReader { buffer, offset: 0 }
    }

    // Reads a specified number of bits from the buffer
    fn read_bits(&mut self, num_bits: usize) -> Result<u64, io::Error> {
        let mut value: u64 = 0;
        let mut bits_read = 0;

        while bits_read < num_bits {
            let byte_index = self.offset / 8;
            let bit_offset = self.offset % 8;

            if byte_index >= self.buffer.len() {
                return Err(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "Buffer underflow while reading bits",
                ));
            }

            let byte = self.buffer[byte_index];
            let bits_to_read = std::cmp::min(num_bits - bits_read, 8 - bit_offset);
            let mask = if bits_to_read >= 8 {
                0xFF
            } else {
                (1u8 << bits_to_read) - 1
            };
            let bits_value = (byte >> (8 - bit_offset - bits_to_read)) & mask;

            value = (value << bits_to_read) | (bits_value as u64);
            self.offset += bits_to_read;
            bits_read += bits_to_read;
        }

        Ok(value)
    }

    // Reads an unsigned integer with a specified number of bits (MSB first)
    fn read_uimsbf(&mut self, num_bits: usize) -> Result<u64, io::Error> {
        self.read_bits(num_bits)
    }

    // Reads an unsigned integer with a specified number of bits (MSB first)
    fn read_bslbf(&mut self, num_bits: usize) -> Result<u64, io::Error> {
        self.read_bits(num_bits)
    }

    // Reads an unsigned integer with a specified number of bits (MSB first)
    // Note: RPCHOF typically implies LSB first within the byte, but SCTE-35 spec
    // doesn't explicitly state this. Assuming standard MSB first based on other fields.
    fn read_rpchof(&mut self, num_bits: usize) -> Result<u64, io::Error> {
        self.read_bits(num_bits)
    }

    // Skips a specified number of bits
    fn skip_bits(&mut self, num_bits: usize) -> Result<(), io::Error> {
        let new_offset = self.offset + num_bits;
        if new_offset / 8 > self.buffer.len() {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Buffer underflow while skipping bits",
            ));
        }
        self.offset = new_offset;
        Ok(())
    }

    // Gets the current bit offset
    fn get_offset(&self) -> usize {
        self.offset
    }
}

// --- SCTE-35 Data Structures ---

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

/// Represents a splice time with optional PTS (Presentation Time Stamp).
///
/// The PTS time is measured in 90kHz ticks, which is the standard timing
/// reference for MPEG transport streams.
#[derive(Debug)]
pub struct SpliceTime {
    /// Indicates whether a specific time is provided (1 = time specified, 0 = immediate)
    pub time_specified_flag: u8,
    /// Presentation timestamp in 90kHz ticks (present when time_specified_flag = 1)
    pub pts_time: Option<u64>,
}

impl SpliceTime {
    /// Converts the PTS time to a [`std::time::Duration`].
    ///
    /// PTS (Presentation Time Stamp) values are stored as 90kHz ticks in SCTE-35 messages.
    /// This method converts those ticks to a standard Rust Duration.
    ///
    /// # Returns
    ///
    /// - `Some(Duration)` if a PTS time is specified
    /// - `None` if no time is specified (time_specified_flag is 0)
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35_parsing::SpliceTime;
    /// use std::time::Duration;
    ///
    /// let splice_time = SpliceTime {
    ///     time_specified_flag: 1,
    ///     pts_time: Some(90_000), // 1 second in 90kHz ticks
    /// };
    ///
    /// let duration = splice_time.to_duration().unwrap();
    /// assert_eq!(duration, Duration::from_secs(1));
    /// ```
    pub fn to_duration(&self) -> Option<Duration> {
        self.pts_time.map(|pts| {
            let seconds = pts / 90_000;
            let nanos = ((pts % 90_000) * 1_000_000_000) / 90_000;
            Duration::new(seconds, nanos as u32)
        })
    }
}

/// Represents a date and time structure used in splice scheduling.
///
/// This structure provides precise timing information for scheduled splice events,
/// including support for both UTC and local time zones.
#[derive(Debug)]
pub struct DateTime {
    /// Indicates if the time is in UTC (1) or local time (0)
    pub utc_flag: u8,
    /// Year value (e.g., 2023)
    pub year: u16,
    /// Month value (1-12)
    pub month: u8,
    /// Day of month (1-31)
    pub day: u8,
    /// Hour value (0-23)
    pub hour: u8,
    /// Minute value (0-59)
    pub minute: u8,
    /// Second value (0-59)
    pub second: u8,
    /// Frame number for sub-second precision
    pub frames: u8,
    /// Millisecond value for additional precision
    pub milliseconds: u8,
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

/// Represents the duration of a commercial break or other timed segment.
///
/// The duration is specified in 90kHz ticks and can optionally indicate
/// whether the break should automatically return to normal programming.
#[derive(Debug)]
pub struct BreakDuration {
    /// Indicates if the break should automatically return to network programming (1 = auto return, 0 = no auto return)
    pub auto_return: u8,
    /// Reserved bits for future use
    pub reserved: u8,
    /// Duration of the break in 90kHz ticks
    pub duration: u64,
}

impl BreakDuration {
    /// Converts the break duration to a [`std::time::Duration`].
    ///
    /// Break durations are stored as 90kHz ticks in SCTE-35 messages.
    /// This method converts those ticks to a standard Rust Duration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35_parsing::BreakDuration;
    /// use std::time::Duration;
    ///
    /// let break_duration = BreakDuration {
    ///     auto_return: 1,
    ///     reserved: 0,
    ///     duration: 2_700_000, // 30 seconds in 90kHz ticks
    /// };
    ///
    /// let duration = break_duration.to_duration();
    /// assert_eq!(duration, Duration::from_secs(30));
    /// ```
    pub fn to_duration(&self) -> Duration {
        let seconds = self.duration / 90_000;
        let nanos = ((self.duration % 90_000) * 1_000_000_000) / 90_000;
        Duration::new(seconds, nanos as u32)
    }
}

impl From<BreakDuration> for Duration {
    fn from(break_duration: BreakDuration) -> Self {
        break_duration.to_duration()
    }
}

impl From<&BreakDuration> for Duration {
    fn from(break_duration: &BreakDuration) -> Self {
        break_duration.to_duration()
    }
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

/// Represents a splice descriptor containing additional metadata.
///
/// Descriptors provide additional information about splice operations,
/// such as segmentation details, UPID (Unique Program Identifier) data,
/// or other custom information.
#[derive(Debug)]
pub struct SpliceDescriptor {
    /// Identifies the type of descriptor (e.g., 0x00 = avail_descriptor, 0x02 = segmentation_descriptor)
    pub descriptor_tag: u8,
    /// Length of the descriptor data in bytes
    pub descriptor_length: u8,
    /// Raw bytes containing the descriptor-specific data
    pub descriptor_bytes: Vec<u8>,
}

impl SpliceDescriptor {
    /// Attempts to interpret the descriptor bytes as a UTF-8 string.
    ///
    /// This is useful for descriptors that contain text-based data,
    /// such as UPID strings or other human-readable information.
    ///
    /// # Returns
    ///
    /// - `Some(&str)` if the bytes form valid UTF-8
    /// - `None` if the bytes are not valid UTF-8
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35_parsing::SpliceDescriptor;
    ///
    /// let descriptor = SpliceDescriptor {
    ///     descriptor_tag: 0x00,
    ///     descriptor_length: 5,
    ///     descriptor_bytes: vec![0x48, 0x65, 0x6c, 0x6c, 0x6f], // "Hello"
    /// };
    ///
    /// assert_eq!(descriptor.as_str(), Some("Hello"));
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.descriptor_bytes).ok()
    }
}

// --- Parsing Functions ---

/// Parses a complete SCTE-35 splice information section from binary data.
///
/// This is the main entry point for parsing SCTE-35 messages. It handles
/// the complete binary format including header fields, splice commands,
/// descriptors, and CRC validation.
///
/// # Arguments
///
/// * `buffer` - A byte slice containing the complete SCTE-35 message
///
/// # Returns
///
/// * `Ok(SpliceInfoSection)` - Successfully parsed SCTE-35 message
/// * `Err(io::Error)` - Parse error (malformed data, buffer underflow, etc.)
///
/// # Supported Command Types
///
/// - `0x00` - Splice Null
/// - `0x04` - Splice Schedule  
/// - `0x05` - Splice Insert
/// - `0x06` - Time Signal
/// - `0x07` - Bandwidth Reservation
/// - `0xFF` - Private Command
///
/// # Example
///
/// ```rust
/// use scte35_parsing::parse_splice_info_section;
/// use base64::{Engine, engine::general_purpose};
///
/// let base64_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
/// let buffer = general_purpose::STANDARD.decode(base64_message).unwrap();
/// 
/// match parse_splice_info_section(&buffer) {
///     Ok(section) => {
///         println!("Successfully parsed SCTE-35 message");
///         println!("Command type: 0x{:02X}", section.splice_command_type);
///     }
///     Err(e) => eprintln!("Parse error: {}", e),
/// }
/// ```
pub fn parse_splice_info_section(buffer: &[u8]) -> Result<SpliceInfoSection, io::Error> {
    let mut reader = BitReader::new(buffer);

    let table_id = reader.read_uimsbf(8)? as u8;
    let section_syntax_indicator = reader.read_bslbf(1)? as u8;
    let private_indicator = reader.read_bslbf(1)? as u8;
    let sap_type = reader.read_bslbf(2)? as u8;
    let section_length = reader.read_uimsbf(12)? as u16;
    let protocol_version = reader.read_uimsbf(8)? as u8;
    let encrypted_packet = reader.read_bslbf(1)? as u8;
    let encryption_algorithm = reader.read_bslbf(6)? as u8;
    let pts_adjustment = reader.read_uimsbf(33)? as u64;
    let cw_index = reader.read_uimsbf(8)? as u8;
    let tier = reader.read_bslbf(12)? as u16;
    let splice_command_length = reader.read_uimsbf(12)? as u16;
    let splice_command_type = reader.read_uimsbf(8)? as u8;

    let command_start_offset = reader.get_offset();
    let splice_command = match splice_command_type {
        0x00 => SpliceCommand::SpliceNull,
        0x04 => SpliceCommand::SpliceSchedule(parse_splice_schedule(&mut reader)?),
        0x05 => SpliceCommand::SpliceInsert(parse_splice_insert(&mut reader)?),
        0x06 => SpliceCommand::TimeSignal(parse_time_signal(&mut reader)?),
        0x07 => SpliceCommand::BandwidthReservation(parse_bandwidth_reservation(&mut reader)?),
        0xff => SpliceCommand::PrivateCommand(parse_private_command(&mut reader)?),
        _ => {
            eprintln!(
                "Warning: Unknown splice_command_type: {}",
                splice_command_type
            );
            // Skip the rest of the command if type is unknown
            reader.skip_bits(splice_command_length as usize * 8)?;
            SpliceCommand::Unknown
        }
    };
    let command_end_offset = reader.get_offset();
    let command_bits_read = command_end_offset - command_start_offset;
    let command_expected_bits = splice_command_length as usize * 8;
    if command_bits_read < command_expected_bits {
        eprintln!(
            "Warning: Splice command length mismatch. Expected {} bits, read {} bits.",
            command_expected_bits, command_bits_read
        );
        reader.skip_bits(command_expected_bits - command_bits_read)?;
    }

    let descriptor_loop_length = reader.read_uimsbf(16)? as u16;
    let mut splice_descriptors = Vec::new();
    let descriptor_start_offset = reader.get_offset();
    let mut descriptor_bits_read = 0;
    while descriptor_bits_read < descriptor_loop_length as usize * 8 {
        splice_descriptors.push(parse_splice_descriptor(&mut reader)?);
        descriptor_bits_read = reader.get_offset() - descriptor_start_offset;
    }
    if descriptor_bits_read > descriptor_loop_length as usize * 8 {
        eprintln!(
            "Warning: Descriptor loop length mismatch. Expected {} bits, read {} bits.",
            descriptor_loop_length as usize * 8,
            descriptor_bits_read
        );
        reader.skip_bits(descriptor_loop_length as usize * 8 - descriptor_bits_read)?;
    }

    // Calculate remaining bits for stuffing
    // The section_length includes everything after the section_length field up to and including the CRC_32
    // So we need to account for the header bytes already read (3 bytes)
    let section_start_bit = 3 * 8; // table_id + flags + section_length = 3 bytes
    let section_end_bit = section_start_bit + (section_length as usize * 8);
    let crc_size_bits = if encrypted_packet == 1 { 64 } else { 32 }; // E_CRC_32 + CRC_32 or just CRC_32
    let expected_content_end = section_end_bit - crc_size_bits;

    let current_offset = reader.get_offset();
    let alignment_stuffing_bits = if current_offset < expected_content_end {
        let remaining_bits = expected_content_end - current_offset;
        let mut stuffing = Vec::new();
        for _ in 0..remaining_bits {
            stuffing.push(reader.read_bslbf(1)? as u8);
        }
        stuffing
    } else {
        Vec::new()
    };

    let e_crc_32 = if encrypted_packet == 1 {
        Some(reader.read_rpchof(32)? as u32)
    } else {
        None
    };
    let crc_32 = reader.read_rpchof(32)? as u32;

    // Validate CRC if feature is enabled - much cleaner!
    #[cfg(feature = "crc-validation")]
    {
        if !crc::validate_crc(&buffer[0..buffer.len() - 4], crc_32) {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("CRC validation failed. Expected: 0x{:08X}", crc_32)
            ));
        }
    }

    Ok(SpliceInfoSection {
        table_id,
        section_syntax_indicator,
        private_indicator,
        sap_type,
        section_length,
        protocol_version,
        encrypted_packet,
        encryption_algorithm,
        pts_adjustment,
        cw_index,
        tier,
        splice_command_length,
        splice_command_type,
        splice_command,
        descriptor_loop_length,
        splice_descriptors,
        alignment_stuffing_bits,
        e_crc_32,
        crc_32,
    })
}

fn parse_splice_schedule(reader: &mut BitReader) -> Result<SpliceSchedule, io::Error> {
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

fn parse_splice_insert(reader: &mut BitReader) -> Result<SpliceInsert, io::Error> {
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

fn parse_time_signal(reader: &mut BitReader) -> Result<TimeSignal, io::Error> {
    let splice_time = parse_splice_time(reader)?;
    Ok(TimeSignal { splice_time })
}

fn parse_bandwidth_reservation(reader: &mut BitReader) -> Result<BandwidthReservation, io::Error> {
    let reserved = reader.read_bslbf(8)? as u8;
    let dwbw_reservation = reader.read_uimsbf(32)? as u32;
    Ok(BandwidthReservation {
        reserved,
        dwbw_reservation,
    })
}

fn parse_private_command(reader: &mut BitReader) -> Result<PrivateCommand, io::Error> {
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

fn parse_splice_time(reader: &mut BitReader) -> Result<SpliceTime, io::Error> {
    let time_specified_flag = reader.read_bslbf(1)? as u8;
    let pts_time = if time_specified_flag == 1 {
        let _reserved = reader.read_bslbf(6)? as u8;
        Some(reader.read_uimsbf(33)? as u64)
    } else {
        let _reserved = reader.read_bslbf(7)? as u8;
        None
    };
    Ok(SpliceTime {
        time_specified_flag,
        pts_time,
    })
}

fn parse_break_duration(reader: &mut BitReader) -> Result<BreakDuration, io::Error> {
    let auto_return = reader.read_bslbf(1)? as u8;
    let reserved = reader.read_bslbf(6)? as u8;
    let duration = reader.read_uimsbf(33)? as u64;
    Ok(BreakDuration {
        auto_return,
        reserved,
        duration,
    })
}

fn parse_date_time(reader: &mut BitReader) -> Result<DateTime, io::Error> {
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

fn parse_component_splice(reader: &mut BitReader) -> Result<ComponentSplice, io::Error> {
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

fn parse_splice_descriptor(reader: &mut BitReader) -> Result<SpliceDescriptor, io::Error> {
    let descriptor_tag = reader.read_uimsbf(8)? as u8;
    let descriptor_length = reader.read_uimsbf(8)? as u8;
    let mut descriptor_bytes = Vec::new();
    for _ in 0..descriptor_length {
        descriptor_bytes.push(reader.read_uimsbf(8)? as u8);
    }
    Ok(SpliceDescriptor {
        descriptor_tag,
        descriptor_length,
        descriptor_bytes,
    })
}

/// Validates the CRC-32 checksum of an SCTE-35 message.
///
/// This is a convenience function that wraps [`crc::validate_message_crc`].
/// For more CRC functionality, use the [`crc`] module directly.
///
/// # Arguments
///
/// * `buffer` - The complete SCTE-35 message bytes
///
/// # Returns
///
/// * `Ok(true)` - CRC validation passed
/// * `Ok(false)` - CRC validation not available (feature disabled)
/// * `Err(io::Error)` - Parse error or validation error
///
/// # Example
///
/// ```rust
/// use scte35_parsing::validate_scte35_crc;
/// use base64::{Engine, engine::general_purpose};
///
/// let base64_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
/// let buffer = general_purpose::STANDARD.decode(base64_message).unwrap();
///
/// match validate_scte35_crc(&buffer) {
///     Ok(true) => println!("CRC validation passed"),
///     Ok(false) => println!("CRC validation failed or not available"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
#[cfg(feature = "crc-validation")]
pub fn validate_scte35_crc(buffer: &[u8]) -> Result<bool, io::Error> {
    crc::validate_message_crc(buffer)
}

/// Stub function when CRC validation is not available.
#[cfg(not(feature = "crc-validation"))]
pub fn validate_scte35_crc(_buffer: &[u8]) -> Result<bool, io::Error> {
    Ok(false) // CRC validation not available
}

impl SpliceInfoSection {
    /// Validates the CRC-32 checksum against the original message data.
    ///
    /// # Arguments
    ///
    /// * `original_buffer` - The original message bytes used to parse this section
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - CRC validation passed
    /// * `Ok(false)` - CRC validation disabled or failed
    /// * `Err(io::Error)` - Validation error
    #[cfg(feature = "crc-validation")]
    pub fn validate_crc(&self, original_buffer: &[u8]) -> Result<bool, io::Error> {
        crc::validate_message_crc(original_buffer)
    }
    
    /// Stub function when CRC validation is not available.
    #[cfg(not(feature = "crc-validation"))]
    pub fn validate_crc(&self, _original_buffer: &[u8]) -> Result<bool, io::Error> {
        Ok(false) // CRC validation not available
    }
    
    /// Returns the stored CRC-32 value from the parsed section.
    pub fn get_crc(&self) -> u32 {
        self.crc_32
    }
}

// Only implement the trait when the feature is available
#[cfg(feature = "crc-validation")]
impl CrcValidatable for SpliceInfoSection {
    fn validate_crc(&self, original_buffer: &[u8]) -> Result<bool, io::Error> {
        crc::validate_message_crc(original_buffer)
    }
    
    fn get_crc(&self) -> u32 {
        self.crc_32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose, Engine};

    #[test]
    fn test_time_signal_command() {
        // Time Signal example from threefive: '/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A=='
        let time_signal_base64 = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = general_purpose::STANDARD
            .decode(time_signal_base64)
            .expect("Failed to decode base64 string");

        let section = parse_splice_info_section(&buffer)
            .expect("Failed to parse time_signal SpliceInfoSection");

        // Validate header
        assert_eq!(section.table_id, 0xFC, "Table ID should be 0xFC");
        assert_eq!(
            section.splice_command_type, 0x06,
            "Command type should be 0x06 (time_signal)"
        );

        // Validate command
        match section.splice_command {
            SpliceCommand::TimeSignal(ref cmd) => {
                assert_eq!(
                    cmd.splice_time.time_specified_flag, 1,
                    "Time should be specified"
                );
                assert!(
                    cmd.splice_time.pts_time.is_some(),
                    "PTS time should be present"
                );

                // Verify time conversion
                if let Some(duration) = cmd.splice_time.to_duration() {
                    // PTS time is 1111111101, which is about 12345 seconds
                    assert!(duration.as_secs() > 12000 && duration.as_secs() < 13000);
                }
            }
            _ => panic!("Expected TimeSignal command"),
        }
    }

    #[test]
    fn test_time_signal_with_descriptors() {
        // Time Signal with descriptors: '/DAgAAAAAAAAAP/wBQb+Qjo1vQAKAAhDVUVJAAAE0iVuWvA='
        let time_signal_desc_base64 = "/DAgAAAAAAAAAP/wBQb+Qjo1vQAKAAhDVUVJAAAE0iVuWvA=";
        let buffer = general_purpose::STANDARD
            .decode(time_signal_desc_base64)
            .expect("Failed to decode base64 string");

        let section = parse_splice_info_section(&buffer)
            .expect("Failed to parse time_signal with descriptors");

        // Validate header
        assert_eq!(section.table_id, 0xFC);
        assert_eq!(
            section.splice_command_type, 0x06,
            "Command type should be 0x06 (time_signal)"
        );

        // Should have descriptors
        assert!(
            section.descriptor_loop_length > 0,
            "Should have descriptors"
        );
        assert!(
            !section.splice_descriptors.is_empty(),
            "Should have descriptor data"
        );
    }

    #[test]
    #[cfg(feature = "crc-validation")]
    fn test_upid_adid_example_invalid_crc() {
        // ADID example with invalid CRC: "/DA4AAAAAAAA///wBQb+AKpFLgAiAiBDVUVJAAAAA3//AAApPWwDDEFCQ0QwMTIzNDU2SHAAAFkTm+A="
        let adid_base64 =
            "/DA4AAAAAAAA///wBQb+AKpFLgAiAiBDVUVJAAAAA3//AAApPWwDDEFCQ0QwMTIzNDU2SHAAAFkTm+A=";
        let buffer = general_purpose::STANDARD
            .decode(adid_base64)
            .expect("Failed to decode ADID base64 string");

        // Should fail to parse due to invalid CRC when CRC validation is enabled
        let section = parse_splice_info_section(&buffer);
        assert!(section.is_err());
        let error = section.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("CRC validation failed"));
    }

    #[test]
    #[cfg(not(feature = "crc-validation"))]
    fn test_upid_adid_example_no_crc_validation() {
        // ADID example (CRC validation disabled): "/DA4AAAAAAAA///wBQb+AKpFLgAiAiBDVUVJAAAAA3//AAApPWwDDEFCQ0QwMTIzNDU2SHAAAFkTm+A="
        let adid_base64 =
            "/DA4AAAAAAAA///wBQb+AKpFLgAiAiBDVUVJAAAAA3//AAApPWwDDEFCQ0QwMTIzNDU2SHAAAFkTm+A=";
        let buffer = general_purpose::STANDARD
            .decode(adid_base64)
            .expect("Failed to decode ADID base64 string");

        // Should parse successfully when CRC validation is disabled
        let section =
            parse_splice_info_section(&buffer).expect("Failed to parse ADID SpliceInfoSection");

        // Validate header
        assert_eq!(section.table_id, 0xFC);
        assert_eq!(
            section.splice_command_type, 0x06,
            "Command type should be 0x06 (time_signal)"
        );

        // Should have descriptors with UPID
        assert!(
            section.descriptor_loop_length > 0,
            "Should have descriptors for UPID"
        );
        assert!(
            !section.splice_descriptors.is_empty(),
            "Should have descriptor data"
        );

        // Check for CUEI descriptor (common in SCTE-35)
        if let Some(first_desc) = section.splice_descriptors.first() {
            assert!(
                first_desc.descriptor_length > 0,
                "Descriptor should have content"
            );
        }
    }

    #[test]
    fn test_upid_umid_example() {
        // UMID example: "/DBDAAAAAAAA///wBQb+AA2QOQAtAitDVUVJAAAAA3+/BCAwNjBhMmIzNC4wMTAxMDEwNS4wMTAxMGQyMC4xEAEBRKI3vg=="
        let umid_base64 = "/DBDAAAAAAAA///wBQb+AA2QOQAtAitDVUVJAAAAA3+/BCAwNjBhMmIzNC4wMTAxMDEwNS4wMTAxMGQyMC4xEAEBRKI3vg==";
        let buffer = general_purpose::STANDARD
            .decode(umid_base64)
            .expect("Failed to decode UMID base64 string");

        let section =
            parse_splice_info_section(&buffer).expect("Failed to parse UMID SpliceInfoSection");

        // Validate header
        assert_eq!(section.table_id, 0xFC);
        assert_eq!(
            section.splice_command_type, 0x06,
            "Command type should be 0x06 (time_signal)"
        );

        // Should have descriptors with UPID
        assert!(
            section.descriptor_loop_length > 0,
            "Should have descriptors for UPID"
        );
        assert!(
            !section.splice_descriptors.is_empty(),
            "Should have descriptor data"
        );
    }

    #[test]
    fn test_upid_isan_example() {
        // ISAN example: "/DA4AAAAAAAA///wBQb+Lom5UgAiAiBDVUVJAAAABn//AAApPWwGDAAAAAA6jQAAAAAAABAAAHGXrpg="
        let isan_base64 =
            "/DA4AAAAAAAA///wBQb+Lom5UgAiAiBDVUVJAAAABn//AAApPWwGDAAAAAA6jQAAAAAAABAAAHGXrpg=";
        let buffer = general_purpose::STANDARD
            .decode(isan_base64)
            .expect("Failed to decode ISAN base64 string");

        let section =
            parse_splice_info_section(&buffer).expect("Failed to parse ISAN SpliceInfoSection");

        // Validate header
        assert_eq!(section.table_id, 0xFC);
        assert_eq!(
            section.splice_command_type, 0x06,
            "Command type should be 0x06 (time_signal)"
        );

        // Should have descriptors with UPID
        assert!(
            section.descriptor_loop_length > 0,
            "Should have descriptors for UPID"
        );
        assert!(
            !section.splice_descriptors.is_empty(),
            "Should have descriptor data"
        );
    }

    #[test]
    fn test_upid_airid_example() {
        // AIRID example: "/DBhAAAAAAAA///wBQb+qM1E7QBLAhdDVUVJSAAArX+fCAgAAAAALLLXnTUCAAIXQ1VFSUgAACZ/nwgIAAAAACyy150RAAACF0NVRUlIAAAnf58ICAAAAAAsstezEAAAihiGnw=="
        let airid_base64 = "/DBhAAAAAAAA///wBQb+qM1E7QBLAhdDVUVJSAAArX+fCAgAAAAALLLXnTUCAAIXQ1VFSUgAACZ/nwgIAAAAACyy150RAAACF0NVRUlIAAAnf58ICAAAAAAsstezEAAAihiGnw==";
        let buffer = general_purpose::STANDARD
            .decode(airid_base64)
            .expect("Failed to decode AIRID base64 string");

        let section =
            parse_splice_info_section(&buffer).expect("Failed to parse AIRID SpliceInfoSection");

        // Validate header
        assert_eq!(section.table_id, 0xFC);
        assert_eq!(
            section.splice_command_type, 0x06,
            "Command type should be 0x06 (time_signal)"
        );

        // Should have multiple descriptors
        assert!(
            section.descriptor_loop_length > 0,
            "Should have descriptors for UPID"
        );
        assert!(
            !section.splice_descriptors.is_empty(),
            "Should have descriptor data"
        );
        assert!(
            section.splice_descriptors.len() >= 3,
            "Should have multiple descriptors"
        );
    }

    #[test]
    fn test_time_signal_placement_opportunity_end() {
        // Time Signal - Placement Opportunity End example
        let placement_end_base64 =
            "/DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=";
        let buffer = general_purpose::STANDARD
            .decode(placement_end_base64)
            .expect("Failed to decode placement opportunity end base64 string");

        let section = parse_splice_info_section(&buffer)
            .expect("Failed to parse placement opportunity end SpliceInfoSection");

        // Validate header
        assert_eq!(section.table_id, 0xFC, "Table ID should be 0xFC");
        assert_eq!(
            section.splice_command_type, 0x06,
            "Command type should be 0x06 (time_signal)"
        );

        // Validate command
        match section.splice_command {
            SpliceCommand::TimeSignal(ref cmd) => {
                assert_eq!(
                    cmd.splice_time.time_specified_flag, 1,
                    "Time should be specified"
                );
                assert!(
                    cmd.splice_time.pts_time.is_some(),
                    "PTS time should be present"
                );

                // Verify time conversion
                if let Some(duration) = cmd.splice_time.to_duration() {
                    // This should represent the end of a placement opportunity
                    assert!(duration.as_secs() > 0, "Duration should be positive");
                }
            }
            _ => panic!("Expected TimeSignal command"),
        }

        // Should have descriptors indicating placement opportunity end
        assert!(
            section.descriptor_loop_length > 0,
            "Should have descriptors for placement opportunity end"
        );
        assert!(
            !section.splice_descriptors.is_empty(),
            "Should have descriptor data"
        );

        // Check for segmentation descriptor (common for placement opportunities)
        if let Some(first_desc) = section.splice_descriptors.first() {
            assert!(
                first_desc.descriptor_length > 0,
                "Descriptor should have content"
            );
            // Descriptor tag 2 is typically segmentation_descriptor
            assert_eq!(
                first_desc.descriptor_tag, 2,
                "Should be segmentation descriptor"
            );
        }
    }

    #[test]
    fn test_multiple_descriptor_types() {
        // Test that we can parse messages with different types of descriptors
        // This demonstrates our parser can handle various SCTE-35 message formats

        // Test 1: Simple time signal (already covered above)
        let time_signal_base64 = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = general_purpose::STANDARD
            .decode(time_signal_base64)
            .unwrap();
        let section = parse_splice_info_section(&buffer).unwrap();
        assert_eq!(section.splice_command_type, 0x06);

        // Test 2: Time signal with descriptors (already covered above)
        let time_signal_desc_base64 = "/DAgAAAAAAAAAP/wBQb+Qjo1vQAKAAhDVUVJAAAE0iVuWvA=";
        let buffer2 = general_purpose::STANDARD
            .decode(time_signal_desc_base64)
            .unwrap();
        let section2 = parse_splice_info_section(&buffer2).unwrap();
        assert_eq!(section2.splice_command_type, 0x06);
        assert!(section2.descriptor_loop_length > 0);

        // Test 3: Complex message with multiple descriptors (AIRID example already covered)
        let complex_base64 = "/DBhAAAAAAAA///wBQb+qM1E7QBLAhdDVUVJSAAArX+fCAgAAAAALLLXnTUCAAIXQ1VFSUgAACZ/nwgIAAAAACyy150RAAACF0NVRUlIAAAnf58ICAAAAAAsstezEAAAihiGnw==";
        let buffer3 = general_purpose::STANDARD.decode(complex_base64).unwrap();
        let section3 = parse_splice_info_section(&buffer3).unwrap();
        assert_eq!(section3.splice_command_type, 0x06);
        assert!(section3.splice_descriptors.len() >= 3);
    }

    #[test]
    fn test_duration_conversions() {
        // Test BreakDuration conversion
        let break_duration = BreakDuration {
            auto_return: 1,
            reserved: 0,
            duration: 5_427_000, // 60.3 seconds in 90kHz ticks
        };

        let duration: Duration = break_duration.to_duration();
        assert_eq!(duration.as_secs(), 60);
        assert_eq!(duration.subsec_millis(), 300);

        // Test using Into trait
        let break_duration2 = BreakDuration {
            auto_return: 1,
            reserved: 0,
            duration: 90_000, // Exactly 1 second
        };

        let duration2: Duration = break_duration2.into();
        assert_eq!(duration2.as_secs(), 1);
        assert_eq!(duration2.subsec_nanos(), 0);

        // Test reference conversion
        let break_duration3 = BreakDuration {
            auto_return: 1,
            reserved: 0,
            duration: 45_000, // 0.5 seconds
        };

        let duration3: Duration = (&break_duration3).into();
        assert_eq!(duration3.as_secs(), 0);
        assert_eq!(duration3.subsec_millis(), 500);

        // Test SpliceTime conversion
        let splice_time = SpliceTime {
            time_specified_flag: 1,
            pts_time: Some(1_935_360_000), // 21504 seconds
        };

        let duration4 = splice_time.to_duration().unwrap();
        assert_eq!(duration4.as_secs(), 21504);
        assert_eq!(duration4.subsec_nanos(), 0);

        // Test SpliceTime with None
        let splice_time_none = SpliceTime {
            time_specified_flag: 0,
            pts_time: None,
        };

        assert!(splice_time_none.to_duration().is_none());
    }

    #[test]
    fn test_splice_descriptor_as_str() {
        // Test with valid UTF-8 bytes
        let descriptor = SpliceDescriptor {
            descriptor_tag: 0x00,
            descriptor_length: 5,
            descriptor_bytes: vec![0x48, 0x65, 0x6c, 0x6c, 0x6f], // "Hello"
        };

        assert_eq!(descriptor.as_str(), Some("Hello"));

        // Test with invalid UTF-8 bytes
        let invalid_descriptor = SpliceDescriptor {
            descriptor_tag: 0x00,
            descriptor_length: 3,
            descriptor_bytes: vec![0xff, 0xfe, 0xfd], // Invalid UTF-8
        };

        assert_eq!(invalid_descriptor.as_str(), None);

        // Test with empty bytes
        let empty_descriptor = SpliceDescriptor {
            descriptor_tag: 0x00,
            descriptor_length: 0,
            descriptor_bytes: vec![],
        };

        assert_eq!(empty_descriptor.as_str(), Some(""));
    }

    #[test]
    fn test_parse_splice_info_section() {
        let example_buffer_base64 =
            "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=";
        let example_buffer = general_purpose::STANDARD
            .decode(example_buffer_base64)
            .expect("Failed to decode base64 string");

        let section =
            parse_splice_info_section(&example_buffer).expect("Failed to parse SpliceInfoSection");

        // Validate header fields
        assert_eq!(section.table_id, 0xFC, "Table ID should be 0xFC");
        assert_eq!(
            section.section_syntax_indicator, 0,
            "Section syntax indicator should be 0 (MPEG Short Section)"
        );
        assert_eq!(
            section.private_indicator, 0,
            "Private indicator should be 0 (Not Private)"
        );
        assert_eq!(section.section_length, 47, "Section length should be 47");
        assert_eq!(section.protocol_version, 0, "Protocol version should be 0");
        assert_eq!(
            section.encrypted_packet, 0,
            "Encrypted packet should be 0 (unencrypted)"
        );
        assert_eq!(
            section.pts_adjustment, 0x000000000,
            "PTS adjustment should be 0x000000000"
        );
        assert_eq!(section.tier, 0xfff, "Tier should be 0xfff");

        // Validate splice command fields
        assert_eq!(
            section.splice_command_length, 0x14,
            "Splice command length should be 0x14"
        );
        assert_eq!(
            section.splice_command_type, 0x05,
            "Splice command type should be 0x05 (SpliceInsert)"
        );

        // Validate SpliceInsert command specifics
        match section.splice_command {
            SpliceCommand::SpliceInsert(ref cmd) => {
                assert_eq!(
                    cmd.splice_event_id, 0x4800008f,
                    "Splice Event ID should be 0x4800008f"
                );
                assert_eq!(
                    cmd.out_of_network_indicator, 1,
                    "Out of network indicator should be 1"
                );
                assert_eq!(
                    cmd.program_splice_flag, 1,
                    "Program splice flag should be 1"
                );
                assert_eq!(cmd.duration_flag, 1, "Duration flag should be 1");
                assert_eq!(
                    cmd.splice_immediate_flag, 0,
                    "Splice immediate flag should be 0"
                );

                // Check splice time
                assert!(cmd.splice_time.is_some(), "Splice time should be present");
                if let Some(splice_time) = &cmd.splice_time {
                    assert_eq!(
                        splice_time.time_specified_flag, 1,
                        "Time specified flag should be 1"
                    );
                    assert_eq!(
                        splice_time.pts_time,
                        Some(0x07369c02e),
                        "PTS time should be 0x07369c02e"
                    );
                }

                // Check break duration
                assert!(
                    cmd.break_duration.is_some(),
                    "Break duration should be present"
                );
                if let Some(break_duration) = &cmd.break_duration {
                    assert_eq!(break_duration.auto_return, 1, "Auto return should be 1");
                    assert_eq!(
                        break_duration.duration, 0x00052ccf5,
                        "Duration should be 0x00052ccf5"
                    );
                }

                assert_eq!(cmd.unique_program_id, 0, "Unique Program ID should be 0");
                assert_eq!(cmd.avail_num, 0, "Avail Num should be 0");
                assert_eq!(cmd.avails_expected, 0, "Avails Expected should be 0");
            }
            _ => panic!("Expected SpliceInsert command"),
        }

        // Validate descriptor loop
        assert_eq!(
            section.descriptor_loop_length, 10,
            "Descriptor loop length should be 10"
        );
        assert_eq!(
            section.splice_descriptors.len(),
            1,
            "Should have 1 descriptor"
        );

        if let Some(descriptor) = section.splice_descriptors.first() {
            assert_eq!(
                descriptor.descriptor_tag, 0x00,
                "Descriptor tag should be 0x00 (Avail Descriptor)"
            );
            assert_eq!(
                descriptor.descriptor_length, 8,
                "Descriptor length should be 8"
            );
            // Validate avail descriptor identifier (first 4 bytes should be 0x00000135)
            assert_eq!(
                descriptor.descriptor_bytes[0], 0x43,
                "First byte should be 0x43"
            );
            assert_eq!(
                descriptor.descriptor_bytes[1], 0x55,
                "Second byte should be 0x55"
            );
            assert_eq!(
                descriptor.descriptor_bytes[2], 0x45,
                "Third byte should be 0x45"
            );
            assert_eq!(
                descriptor.descriptor_bytes[3], 0x49,
                "Fourth byte should be 0x49"
            );
            assert_eq!(
                descriptor.descriptor_bytes[4], 0x00,
                "Fifth byte should be 0x00"
            );
            assert_eq!(
                descriptor.descriptor_bytes[5], 0x00,
                "Sixth byte should be 0x00"
            );
            assert_eq!(
                descriptor.descriptor_bytes[6], 0x01,
                "Seventh byte should be 0x01"
            );
            assert_eq!(
                descriptor.descriptor_bytes[7], 0x35,
                "Eighth byte should be 0x35"
            );
        }
    }

    #[test]
    #[cfg(feature = "crc-validation")]
    fn test_valid_crc() {
        let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = general_purpose::STANDARD.decode(valid_message).unwrap();
        
        let result = validate_scte35_crc(&buffer);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    #[cfg(feature = "crc-validation")]
    fn test_invalid_crc() {
        let mut buffer = general_purpose::STANDARD
            .decode("/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==")
            .unwrap();
        
        // Corrupt the CRC (last 4 bytes)
        let len = buffer.len();
        buffer[len - 1] = 0x00;
        
        let result = validate_scte35_crc(&buffer);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    #[cfg(feature = "crc-validation")]
    fn test_parse_with_crc_validation() {
        let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = general_purpose::STANDARD.decode(valid_message).unwrap();
        
        // Should parse successfully with valid CRC
        let section = parse_splice_info_section(&buffer);
        assert!(section.is_ok());
    }

    #[test]
    #[cfg(feature = "crc-validation")]
    fn test_parse_with_invalid_crc_fails() {
        let mut buffer = general_purpose::STANDARD
            .decode("/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==")
            .unwrap();
        
        // Corrupt the CRC (last 4 bytes)
        let len = buffer.len();
        buffer[len - 1] = 0x00;
        
        // Should fail to parse with invalid CRC
        let section = parse_splice_info_section(&buffer);
        assert!(section.is_err());
        let error = section.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("CRC validation failed"));
    }

    #[test]
    #[cfg(feature = "crc-validation")]
    fn test_splice_info_section_validate_crc() {
        let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = general_purpose::STANDARD.decode(valid_message).unwrap();
        
        let section = parse_splice_info_section(&buffer).unwrap();
        
        // Test method-based validation
        let result = section.validate_crc(&buffer);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Test get_crc method
        assert_eq!(section.get_crc(), section.crc_32);
    }

    #[test]
    #[cfg(feature = "crc-validation")]
    fn test_crc_validatable_trait() {
        let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = general_purpose::STANDARD.decode(valid_message).unwrap();
        
        let section = parse_splice_info_section(&buffer).unwrap();
        
        // Test trait implementation
        let result = section.validate_crc(&buffer);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        let crc = section.get_crc();
        assert!(crc > 0);
    }

    #[test]
    #[cfg(not(feature = "crc-validation"))]
    fn test_crc_disabled() {
        let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = general_purpose::STANDARD.decode(valid_message).unwrap();
        
        // Should always return false when CRC validation is disabled
        let result = validate_scte35_crc(&buffer);
        assert!(result.is_ok());
        assert!(!result.unwrap());
        
        // Parse should still work without CRC validation
        let section = parse_splice_info_section(&buffer);
        assert!(section.is_ok());
        
        // Method should return false when disabled
        let section = section.unwrap();
        let result = section.validate_crc(&buffer);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
