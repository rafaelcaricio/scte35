//! Main parsing functions for SCTE-35 messages.
//!
//! This module contains the primary parsing logic for SCTE-35 splice information sections
//! and related structures.

use crate::bit_reader::BitReader;
use crate::commands::parse_splice_command;
use crate::descriptors::{SegmentationDescriptor, SpliceDescriptor};
use crate::types::{SegmentationType, SpliceInfoSection};
use crate::upid::SegmentationUpidType;
use std::io::{self, ErrorKind};

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
    let pts_adjustment = reader.read_uimsbf(33)?;
    let cw_index = reader.read_uimsbf(8)? as u8;
    let tier = reader.read_bslbf(12)? as u16;
    let splice_command_length = reader.read_uimsbf(12)? as u16;
    let splice_command_type = reader.read_uimsbf(8)? as u8;

    let command_start_offset = reader.get_offset();
    let splice_command =
        parse_splice_command(&mut reader, splice_command_type, splice_command_length)?;
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
        if !crate::crc::validate_crc(&buffer[0..buffer.len() - 4], crc_32) {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("CRC validation failed. Expected: 0x{:08X}", crc_32),
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

/// Parses a splice descriptor from the bit stream.
pub(crate) fn parse_splice_descriptor(
    reader: &mut BitReader,
) -> Result<SpliceDescriptor, io::Error> {
    let descriptor_tag = reader.read_uimsbf(8)? as u8;
    let descriptor_length = reader.read_uimsbf(8)? as u8;

    match descriptor_tag {
        0x02 => {
            // Segmentation descriptor - parse it fully
            let segmentation_descriptor = parse_segmentation_descriptor(reader, descriptor_length)?;
            Ok(SpliceDescriptor::Segmentation(segmentation_descriptor))
        }
        _ => {
            // Unknown descriptor - store raw bytes
            let mut descriptor_bytes = Vec::new();
            for _ in 0..descriptor_length {
                descriptor_bytes.push(reader.read_uimsbf(8)? as u8);
            }
            Ok(SpliceDescriptor::Unknown {
                tag: descriptor_tag,
                length: descriptor_length,
                data: descriptor_bytes,
            })
        }
    }
}

/// Parses a segmentation descriptor from the bit stream.
///
/// This function implements the complete SCTE-35 segmentation descriptor parsing
/// according to the specification, including all conditional fields and UPID parsing.
/// It carefully tracks bytes read to avoid buffer underflow.
pub(crate) fn parse_segmentation_descriptor(
    reader: &mut BitReader,
    descriptor_length: u8,
) -> Result<SegmentationDescriptor, io::Error> {
    let start_offset = reader.get_offset();
    let max_bits = descriptor_length as usize * 8;

    // First, validate the mandatory CUEI identifier (4 bytes)
    if max_bits < 32 {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "Segmentation descriptor too short for CUEI identifier",
        ));
    }

    let identifier = reader.read_uimsbf(32)? as u32;
    if identifier != 0x43554549 {
        // "CUEI" in big-endian
        return Err(io::Error::new(ErrorKind::InvalidData,
            format!("Invalid segmentation descriptor identifier: expected 0x43554549 (CUEI), got 0x{:08x}", identifier)));
    }

    // Read the segmentation event fields (5 bytes minimum after CUEI)
    if (reader.get_offset() - start_offset) + 40 > max_bits {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "Segmentation descriptor too short for event fields",
        ));
    }

    let segmentation_event_id = reader.read_uimsbf(32)? as u32;
    let segmentation_event_cancel_indicator = reader.read_bslbf(1)? != 0;
    let _reserved = reader.read_bslbf(7)?; // reserved bits

    if segmentation_event_cancel_indicator {
        // If cancel indicator is set, only the event ID and cancel flag are present
        return Ok(SegmentationDescriptor {
            segmentation_event_id,
            segmentation_event_cancel_indicator: true,
            program_segmentation_flag: false,
            segmentation_duration_flag: false,
            delivery_not_restricted_flag: false,
            web_delivery_allowed_flag: None,
            no_regional_blackout_flag: None,
            archive_allowed_flag: None,
            device_restrictions: None,
            segmentation_duration: None,
            segmentation_upid_type: SegmentationUpidType::NotUsed,
            segmentation_upid_length: 0,
            segmentation_upid: Vec::new(),
            segmentation_type_id: 0,
            segmentation_type: SegmentationType::from_id(0),
            segment_num: 0,
            segments_expected: 0,
            sub_segment_num: None,
            sub_segments_expected: None,
        });
    }

    // Check if we have enough bits for the next byte
    if (reader.get_offset() - start_offset) + 8 > max_bits {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "Segmentation descriptor too short",
        ));
    }

    let program_segmentation_flag = reader.read_bslbf(1)? != 0;
    let segmentation_duration_flag = reader.read_bslbf(1)? != 0;
    let delivery_not_restricted_flag = reader.read_bslbf(1)? != 0;

    let (
        web_delivery_allowed_flag,
        no_regional_blackout_flag,
        archive_allowed_flag,
        device_restrictions,
    ) = if !delivery_not_restricted_flag {
        let web_delivery_allowed = reader.read_bslbf(1)? != 0;
        let no_regional_blackout = reader.read_bslbf(1)? != 0;
        let archive_allowed = reader.read_bslbf(1)? != 0;
        let device_restrictions = reader.read_bslbf(2)? as u8;
        (
            Some(web_delivery_allowed),
            Some(no_regional_blackout),
            Some(archive_allowed),
            Some(device_restrictions),
        )
    } else {
        let _reserved = reader.read_bslbf(5)?; // reserved bits when delivery not restricted
        (None, None, None, None)
    };

    // Handle component data if program_segmentation_flag is false
    if !program_segmentation_flag {
        if (reader.get_offset() - start_offset) + 8 > max_bits {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Segmentation descriptor too short for component count",
            ));
        }
        let component_count = reader.read_uimsbf(8)? as u8;

        // Each component is 6 bytes (48 bits)
        let component_data_bits = component_count as usize * 48;
        if (reader.get_offset() - start_offset) + component_data_bits > max_bits {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Segmentation descriptor too short for component data",
            ));
        }

        // Skip component data
        for _ in 0..component_count {
            let _component_tag = reader.read_uimsbf(8)?;
            let _reserved = reader.read_bslbf(7)?;
            let _pts_offset = reader.read_uimsbf(33)?;
        }
    }

    // Read segmentation duration if present (5 bytes)
    let segmentation_duration = if segmentation_duration_flag {
        if (reader.get_offset() - start_offset) + 40 > max_bits {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Segmentation descriptor too short for duration",
            ));
        }
        Some(reader.read_uimsbf(40)?)
    } else {
        None
    };

    // Read UPID type and length (2 bytes minimum)
    if (reader.get_offset() - start_offset) + 16 > max_bits {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "Segmentation descriptor too short for UPID header",
        ));
    }

    let segmentation_upid_type_byte = reader.read_uimsbf(8)? as u8;
    let segmentation_upid_type = SegmentationUpidType::from(segmentation_upid_type_byte);
    let segmentation_upid_length = reader.read_uimsbf(8)? as u8;

    // Read UPID data - cap to available bytes, accounting for minimum 3 bytes needed after UPID
    let current_bits_used = reader.get_offset() - start_offset;
    let remaining_bits = max_bits - current_bits_used;
    let min_bits_after_upid = 24; // 3 bytes for segmentation_type_id, segment_num, segments_expected
    let max_upid_bits = remaining_bits.saturating_sub(min_bits_after_upid);
    let max_upid_bytes = max_upid_bits / 8;
    let actual_upid_length = std::cmp::min(segmentation_upid_length as usize, max_upid_bytes);

    let mut segmentation_upid = Vec::new();
    for _ in 0..actual_upid_length {
        segmentation_upid.push(reader.read_uimsbf(8)? as u8);
    }

    // Read segmentation type, segment num, and segments expected (3 bytes)
    if (reader.get_offset() - start_offset) + 24 > max_bits {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "Segmentation descriptor too short for segmentation fields",
        ));
    }

    let segmentation_type_id = reader.read_uimsbf(8)? as u8;
    let segment_num = reader.read_uimsbf(8)? as u8;
    let segments_expected = reader.read_uimsbf(8)? as u8;

    // Sub-segment fields are present for certain segmentation types (2 additional bytes)
    let (sub_segment_num, sub_segments_expected) = match segmentation_type_id {
        0x34 | 0x36 | 0x38 | 0x3A => {
            if (reader.get_offset() - start_offset) + 16 <= max_bits {
                let sub_segment_num = reader.read_uimsbf(8)? as u8;
                let sub_segments_expected = reader.read_uimsbf(8)? as u8;
                (Some(sub_segment_num), Some(sub_segments_expected))
            } else {
                // Not enough bytes for sub-segment fields
                (None, None)
            }
        }
        _ => (None, None),
    };

    Ok(SegmentationDescriptor {
        segmentation_event_id,
        segmentation_event_cancel_indicator,
        program_segmentation_flag,
        segmentation_duration_flag,
        delivery_not_restricted_flag,
        web_delivery_allowed_flag,
        no_regional_blackout_flag,
        archive_allowed_flag,
        device_restrictions,
        segmentation_duration,
        segmentation_upid_type,
        segmentation_upid_length: actual_upid_length as u8,
        segmentation_upid,
        segmentation_type_id,
        segmentation_type: SegmentationType::from_id(segmentation_type_id),
        segment_num,
        segments_expected,
        sub_segment_num,
        sub_segments_expected,
    })
}
