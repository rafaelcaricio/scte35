use crate::commands::SpliceCommand;
use crate::descriptors::SpliceDescriptor;
use crate::{CueError, TransportPacketWrite};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use std::io;
use crc::{Crc, Algorithm, CRC_32_MPEG_2};

pub const MPEG_2: Crc<u32> = Crc::<u32>::new(&CRC_32_MPEG_2);

pub struct SpliceInfoSection<C>
where
    C: SpliceCommand,
{
    /// This is an 8-bit field. Its value shall be 0xFC.
    table_id: u8,

    /// The section_syntax_indicator is a 1-bit field that should always be set to ‘0’, indicating
    /// that MPEG short sections are to be used.
    section_syntax_indicator: bool,

    /// This is a 1-bit flag that shall be set to 0.
    private_indicator: bool,

    /// A two-bit field that indicates if the content preparation system has created a Stream
    /// Access Point (SAP) at the signaled point in the stream. SAP types are defined in
    /// ISO 14496-12, Annex I. The semantics of SAP types are further informatively elaborated
    /// in ISO/IEC 23009-1 DASH, Section 4.5.2.
    sap_type: SAPType, // 2 bits

    protocol_version: u8,
    encrypted_packet: bool,
    encryption_algorithm: EncryptionAlgorithm,
    pts_adjustment: u64, // 33 bits
    cw_index: u8,
    tier: u16, // 12 bits

    splice_command: C,

    descriptors: Vec<SpliceDescriptor>,
}

impl<C> SpliceInfoSection<C>
where
    C: SpliceCommand,
{
    fn new(splice_command: C) -> Self {
        Self {
            table_id: 0xFC,
            section_syntax_indicator: false,
            private_indicator: false,
            sap_type: SAPType::NotSpecified,
            protocol_version: 0,
            encrypted_packet: false,
            encryption_algorithm: EncryptionAlgorithm::NotEncrypted,
            pts_adjustment: 0,
            cw_index: 0,
            tier: 0xFFF,
            splice_command,
            descriptors: Vec::new(),
        }
    }

    pub fn as_base64(&self) -> Result<String, CueError> {
        let mut out = Vec::new();
        self.write_to(&mut out)?;
        Ok(base64::encode(out.as_slice()))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum SAPType {
    Type1 = 0x00,
    Type2 = 0x01,
    Type3 = 0x02,
    NotSpecified = 0x03,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EncryptionAlgorithm {
    NotEncrypted,
    DESECBMode,
    DESCBCMode,
    TripleDESEDE3ECBMode,
    Reserved(u8), // 4-31
    Private(u8),  // 32-63
}

impl From<u8> for EncryptionAlgorithm {
    fn from(value: u8) -> Self {
        match value {
            0x00 => EncryptionAlgorithm::NotEncrypted,
            0x01 => EncryptionAlgorithm::DESECBMode,
            0x02 => EncryptionAlgorithm::DESCBCMode,
            0x03 => EncryptionAlgorithm::TripleDESEDE3ECBMode,
            0x04..=0x1F => EncryptionAlgorithm::Reserved(value),
            _ => EncryptionAlgorithm::Private(value),
        }
    }
}

impl From<EncryptionAlgorithm> for u8 {
    fn from(value: EncryptionAlgorithm) -> Self {
        match value {
            EncryptionAlgorithm::NotEncrypted => 0x00,
            EncryptionAlgorithm::DESECBMode => 0x01,
            EncryptionAlgorithm::DESCBCMode => 0x02,
            EncryptionAlgorithm::TripleDESEDE3ECBMode => 0x03,
            EncryptionAlgorithm::Reserved(value) => value,
            EncryptionAlgorithm::Private(value) => value,
        }
    }
}

impl<C> TransportPacketWrite for SpliceInfoSection<C>
where
    C: SpliceCommand,
{
    fn write_to<W>(&self, out: &mut W) -> Result<(), CueError>
    where
        W: io::Write,
    {
        // Write splice command to a temporary buffer
        let mut splice_data = Vec::new();
        self.splice_command.write_to(&mut splice_data)?;

        // Write the descriptors to a temporary buffer
        let mut descriptor_data = Vec::new();
        for descriptor in &self.descriptors {
            descriptor.write_to(&mut descriptor_data)?;
        }

        // Start writing the final output to a temporary buffer
        let mut data = Vec::new();
        let mut buffer = BitWriter::endian(&mut data, BigEndian);
        buffer.write(8, self.table_id)?;
        buffer.write_bit(self.section_syntax_indicator)?;
        buffer.write_bit(self.private_indicator)?;
        buffer.write(2, self.sap_type as u8)?;

        // We know the section length by computing all known fixed size elements from now plus the
        // splice command length and descriptors which are also known by now
        const FIXED_INFO_SIZE_BYTES: usize = (8 + 1 + 6 + 33 + 8 + 12 + 12 + 8 + 16 + 32) / 8;
        let mut section_length = FIXED_INFO_SIZE_BYTES + splice_data.len() + descriptor_data.len();
        if self.encrypted_packet {
            section_length += 4;
        }
        buffer.write(12, section_length as u16)?;
        buffer.write(8, self.protocol_version)?;
        buffer.write_bit(self.encrypted_packet)?;
        let encryption_algorithm: u8 = self.encryption_algorithm.into();
        buffer.write(6, encryption_algorithm)?;
        buffer.write(33, self.pts_adjustment)?;
        buffer.write(8, self.cw_index)?;
        buffer.write(12, self.tier)?;
        buffer.write(12, splice_data.len() as u16)?;
        buffer.write(8, self.splice_command.splice_command_type())?;
        buffer.write_bytes(splice_data.as_slice())?;
        buffer.write(16, descriptor_data.len() as u16)?;
        buffer.write_bytes(descriptor_data.as_slice())?;
        buffer.flush()?;

        // Finally, write to out
        let mut buffer = BitWriter::endian(out, BigEndian);
        buffer.write_bytes(data.as_slice())?;
        // CRC 32
        if self.encrypted_packet {
            // TODO: alignment stuffing here, in case of DES encryption this needs to be 8 bytes aligned
            // encrypted_packet_crc32:
            buffer.write(32, 0)?;
        }
        // TODO: Calculate CRC32. Use the data information
        // crc32:
        buffer.write(32, MPEG_2.checksum(data.as_slice()))?;
        buffer.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::SpliceNull;

    #[test]
    fn write_null_splice() {
        let splice = SpliceInfoSection::new(SpliceNull::new());

        assert_eq!(splice.as_base64().unwrap(), "/DARAAAAAAAAAP/wAAAAAHpPv/8=".to_string());
    }
}
