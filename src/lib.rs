use std::io::{self, ErrorKind};
use std::time::Duration;

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

#[derive(Debug)]
pub struct SpliceInfoSection {
    pub table_id: u8,
    pub section_syntax_indicator: u8,
    pub private_indicator: u8,
    pub sap_type: u8,
    pub section_length: u16,
    pub protocol_version: u8,
    pub encrypted_packet: u8,
    pub encryption_algorithm: u8,
    pub pts_adjustment: u64,
    pub cw_index: u8,
    pub tier: u16,
    pub splice_command_length: u16,
    pub splice_command_type: u8,
    pub splice_command: SpliceCommand,
    pub descriptor_loop_length: u16,
    pub splice_descriptors: Vec<SpliceDescriptor>,
    pub alignment_stuffing_bits: Vec<u8>,
    pub e_crc_32: Option<u32>,
    pub crc_32: u32,
}

#[derive(Debug)]
pub enum SpliceCommand {
    SpliceNull,
    SpliceSchedule(SpliceSchedule),
    SpliceInsert(SpliceInsert),
    TimeSignal(TimeSignal),
    BandwidthReservation(BandwidthReservation),
    PrivateCommand(PrivateCommand),
    Unknown,
}

#[derive(Debug)]
pub struct SpliceNull {}

#[derive(Debug)]
pub struct SpliceSchedule {
    pub splice_event_id: u32,
    pub splice_event_cancel_indicator: u8,
    pub reserved: u8,
    pub out_of_network_indicator: u8,
    pub duration_flag: u8,
    pub splice_duration: Option<u32>,
    pub scheduled_splice_time: Option<DateTime>,
    pub unique_program_id: u16,
    pub num_splice: u8,
    pub component_list: Vec<ComponentSplice>,
}

#[derive(Debug)]
pub struct SpliceInsert {
    pub splice_event_id: u32,
    pub splice_event_cancel_indicator: u8,
    pub reserved: u8,
    pub out_of_network_indicator: u8,
    pub program_splice_flag: u8,
    pub duration_flag: u8,
    pub splice_immediate_flag: u8,
    pub reserved2: u8,
    pub splice_time: Option<SpliceTime>,
    pub component_count: u8,
    pub components: Vec<SpliceInsertComponent>,
    pub break_duration: Option<BreakDuration>,
    pub unique_program_id: u16,
    pub avail_num: u8,
    pub avails_expected: u8,
}

#[derive(Debug)]
pub struct TimeSignal {
    pub splice_time: SpliceTime,
}

#[derive(Debug)]
pub struct BandwidthReservation {
    pub reserved: u8,
    pub dwbw_reservation: u32,
}

#[derive(Debug)]
pub struct PrivateCommand {
    pub private_command_id: u16,
    pub private_command_length: u8,
    pub private_bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct SpliceTime {
    pub time_specified_flag: u8,
    pub pts_time: Option<u64>,
}

impl SpliceTime {
    /// Convert PTS time to Duration (PTS is in 90kHz ticks)
    pub fn to_duration(&self) -> Option<Duration> {
        self.pts_time.map(|pts| {
            let seconds = pts / 90_000;
            let nanos = ((pts % 90_000) * 1_000_000_000) / 90_000;
            Duration::new(seconds, nanos as u32)
        })
    }
}

#[derive(Debug)]
pub struct DateTime {
    pub utc_flag: u8,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub frames: u8,
    pub milliseconds: u8,
}

#[derive(Debug)]
pub struct ComponentSplice {
    pub component_tag: u8,
    pub reserved: u8,
    pub splice_mode_indicator: u8,
    pub duration_flag: u8,
    pub splice_duration: Option<u32>,
    pub scheduled_splice_time: Option<DateTime>,
}

#[derive(Debug)]
pub struct BreakDuration {
    pub auto_return: u8,
    pub reserved: u8,
    pub duration: u64,
}

impl BreakDuration {
    /// Convert duration to Duration (duration is in 90kHz ticks)
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

#[derive(Debug)]
pub struct SpliceInsertComponent {
    pub component_tag: u8,
    pub splice_time: Option<SpliceTime>,
}

#[derive(Debug)]
pub struct SpliceDescriptor {
    pub descriptor_tag: u8,
    pub descriptor_length: u8,
    pub descriptor_bytes: Vec<u8>,
}

// --- Parsing Functions ---

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
        let reserved = reader.read_bslbf(5)? as u8;
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine, engine::general_purpose};

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
    fn test_parse_splice_info_section() {
        let example_buffer_base64 =
            "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=";
        let example_buffer = general_purpose::STANDARD
            .decode(example_buffer_base64)
            .expect("Failed to decode base64 string");

        let section = parse_splice_info_section(&example_buffer)
            .expect("Failed to parse SpliceInfoSection");

        // Validate header fields
        assert_eq!(section.table_id, 0xFC, "Table ID should be 0xFC");
        assert_eq!(section.section_syntax_indicator, 0, "Section syntax indicator should be 0 (MPEG Short Section)");
        assert_eq!(section.private_indicator, 0, "Private indicator should be 0 (Not Private)");
        assert_eq!(section.section_length, 47, "Section length should be 47");
        assert_eq!(section.protocol_version, 0, "Protocol version should be 0");
        assert_eq!(section.encrypted_packet, 0, "Encrypted packet should be 0 (unencrypted)");
        assert_eq!(section.pts_adjustment, 0x000000000, "PTS adjustment should be 0x000000000");
        assert_eq!(section.tier, 0xfff, "Tier should be 0xfff");
        
        // Validate splice command fields
        assert_eq!(section.splice_command_length, 0x14, "Splice command length should be 0x14");
        assert_eq!(section.splice_command_type, 0x05, "Splice command type should be 0x05 (SpliceInsert)");
        
        // Validate SpliceInsert command specifics
        match section.splice_command {
            SpliceCommand::SpliceInsert(ref cmd) => {
                assert_eq!(cmd.splice_event_id, 0x4800008f, "Splice Event ID should be 0x4800008f");
                assert_eq!(cmd.out_of_network_indicator, 1, "Out of network indicator should be 1");
                assert_eq!(cmd.program_splice_flag, 1, "Program splice flag should be 1");
                assert_eq!(cmd.duration_flag, 1, "Duration flag should be 1");
                assert_eq!(cmd.splice_immediate_flag, 0, "Splice immediate flag should be 0");
                
                // Check splice time
                assert!(cmd.splice_time.is_some(), "Splice time should be present");
                if let Some(splice_time) = &cmd.splice_time {
                    assert_eq!(splice_time.time_specified_flag, 1, "Time specified flag should be 1");
                    assert_eq!(splice_time.pts_time, Some(0x07369c02e), "PTS time should be 0x07369c02e");
                }
                
                // Check break duration
                assert!(cmd.break_duration.is_some(), "Break duration should be present");
                if let Some(break_duration) = &cmd.break_duration {
                    assert_eq!(break_duration.auto_return, 1, "Auto return should be 1");
                    assert_eq!(break_duration.duration, 0x00052ccf5, "Duration should be 0x00052ccf5");
                }
                
                assert_eq!(cmd.unique_program_id, 0, "Unique Program ID should be 0");
                assert_eq!(cmd.avail_num, 0, "Avail Num should be 0");
                assert_eq!(cmd.avails_expected, 0, "Avails Expected should be 0");
            }
            _ => panic!("Expected SpliceInsert command"),
        }
        
        // Validate descriptor loop
        assert_eq!(section.descriptor_loop_length, 10, "Descriptor loop length should be 10");
        assert_eq!(section.splice_descriptors.len(), 1, "Should have 1 descriptor");
        
        if let Some(descriptor) = section.splice_descriptors.first() {
            assert_eq!(descriptor.descriptor_tag, 0x00, "Descriptor tag should be 0x00 (Avail Descriptor)");
            assert_eq!(descriptor.descriptor_length, 8, "Descriptor length should be 8");
            // Validate avail descriptor identifier (first 4 bytes should be 0x00000135)
            assert_eq!(descriptor.descriptor_bytes[0], 0x43, "First byte should be 0x43");
            assert_eq!(descriptor.descriptor_bytes[1], 0x55, "Second byte should be 0x55");
            assert_eq!(descriptor.descriptor_bytes[2], 0x45, "Third byte should be 0x45");
            assert_eq!(descriptor.descriptor_bytes[3], 0x49, "Fourth byte should be 0x49");
            assert_eq!(descriptor.descriptor_bytes[4], 0x00, "Fifth byte should be 0x00");
            assert_eq!(descriptor.descriptor_bytes[5], 0x00, "Sixth byte should be 0x00");
            assert_eq!(descriptor.descriptor_bytes[6], 0x01, "Seventh byte should be 0x01");
            assert_eq!(descriptor.descriptor_bytes[7], 0x35, "Eighth byte should be 0x35");
        }
    }
}
