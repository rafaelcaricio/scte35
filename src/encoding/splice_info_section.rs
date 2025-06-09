//! Encoding implementation for SpliceInfoSection.

use crate::types::SpliceInfoSection;
use crate::encoding::{BitWriter, Encodable, EncodingResult};

impl SpliceInfoSection {
    /// Calculate the correct section_length for encoding.
    fn calculate_section_length(&self) -> u16 {
        // Section length is from the byte after section_length to the end (including CRC)
        // Total size minus the first 3 bytes (table_id + section_syntax_indicator/private_indicator/sap_type + section_length)
        // The encoded_size method calculates total size, so we subtract 3
        (self.encoded_size() - 3) as u16
    }
    
    /// Calculate the correct splice_command_length for encoding.
    fn calculate_splice_command_length(&self) -> u16 {
        self.splice_command.encoded_size() as u16
    }
    
    /// Encode all fields except the CRC-32.
    fn encode_without_crc(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // Table ID (8 bits)
        writer.write_bits(self.table_id as u64, 8)?;
        
        // Section syntax indicator (1 bit)
        writer.write_bits(self.section_syntax_indicator as u64, 1)?;
        
        // Private indicator (1 bit)
        writer.write_bits(self.private_indicator as u64, 1)?;
        
        // SAP type (2 bits)
        writer.write_bits(self.sap_type as u64, 2)?;
        
        // Section length (12 bits) - calculate the correct value
        let section_length = self.calculate_section_length();
        writer.write_bits(section_length as u64, 12)?;
        
        // Protocol version (8 bits)
        writer.write_bits(self.protocol_version as u64, 8)?;
        
        // Encrypted packet (1 bit)
        writer.write_bits(self.encrypted_packet as u64, 1)?;
        
        // Encryption algorithm (6 bits)
        writer.write_bits(self.encryption_algorithm as u64, 6)?;
        
        // PTS adjustment (33 bits)
        writer.write_bits(self.pts_adjustment & 0x1FFFFFFFF, 33)?;
        
        // CW index (8 bits)
        writer.write_bits(self.cw_index as u64, 8)?;
        
        // Tier (12 bits)
        writer.write_bits(self.tier as u64 & 0xFFF, 12)?;
        
        // Splice command length (12 bits) - calculate the correct value
        let splice_command_length = self.calculate_splice_command_length();
        writer.write_bits(splice_command_length as u64, 12)?;
        
        // Splice command type (8 bits)
        writer.write_bits(self.splice_command_type as u64, 8)?;
        
        // Encode splice command
        self.splice_command.encode(writer)?;
        
        // Descriptor loop length (16 bits) - calculate the correct value
        let mut descriptor_loop_length = 0u16;
        for descriptor in &self.splice_descriptors {
            descriptor_loop_length += descriptor.encoded_size() as u16;
        }
        writer.write_bits(descriptor_loop_length as u64, 16)?;
        
        // Encode splice descriptors
        for descriptor in &self.splice_descriptors {
            descriptor.encode(writer)?;
        }
        
        // Alignment stuffing
        if !self.alignment_stuffing_bits.is_empty() {
            writer.write_bytes(&self.alignment_stuffing_bits)?;
        }
        
        // E_CRC_32 if encrypted
        if let Some(e_crc) = self.e_crc_32 {
            writer.write_bits(e_crc as u64, 32)?;
        }
        
        Ok(())
    }
}

impl Encodable for SpliceInfoSection {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // Encode everything except CRC
        self.encode_without_crc(writer)?;
        
        // CRC-32 (placeholder for now, will be calculated later)
        writer.write_bits(self.crc_32 as u64, 32)?;
        
        Ok(())
    }
    
    fn encoded_size(&self) -> usize {
        // Fixed header size (up to and including splice_command_type)
        // 112 bits = 14 bytes exactly
        let mut size = 14; // bytes
        
        // Splice command size
        size += self.splice_command.encoded_size();
        
        // Descriptor loop length field
        size += 2;
        
        // Descriptors
        for descriptor in &self.splice_descriptors {
            size += descriptor.encoded_size();
        }
        
        // Alignment stuffing
        size += self.alignment_stuffing_bits.len();
        
        // E_CRC_32 if present
        if self.e_crc_32.is_some() {
            size += 4;
        }
        
        // CRC_32
        size += 4;
        
        size
    }
}

#[cfg(feature = "crc-validation")]
use crate::encoding::CrcEncodable;

#[cfg(feature = "crc-validation")]
impl CrcEncodable for SpliceInfoSection {
    fn encode_with_crc(&self) -> EncodingResult<Vec<u8>> {
        use crate::crc::calculate_crc;
        
        // Encode everything except the CRC field
        let mut writer = BitWriter::with_capacity(self.encoded_size());
        
        // Encode all fields up to CRC
        self.encode_without_crc(&mut writer)?;
        
        // Get the buffer and calculate CRC
        let mut buffer = writer.finish();
        
        if let Some(crc) = calculate_crc(&buffer) {
            // Append the calculated CRC
            buffer.extend_from_slice(&crc.to_be_bytes());
        } else {
            // If CRC calculation is not available, use the stored CRC
            buffer.extend_from_slice(&self.crc_32.to_be_bytes());
        }
        
        Ok(buffer)
    }
}

#[cfg(feature = "base64")]
use crate::encoding::Base64Encodable;

#[cfg(feature = "base64")]
impl Base64Encodable for SpliceInfoSection {}