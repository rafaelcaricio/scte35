use crate::commands::{SpliceCommand, SpliceCommandType};
use crate::descriptors::SpliceDescriptor;
use crate::{CueError, TransportPacketWrite};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use crc::{Crc, CRC_32_MPEG_2};
use std::fmt::{Display, Formatter};

pub const MPEG_2: Crc<u32> = Crc::<u32>::new(&CRC_32_MPEG_2);

pub struct SpliceInfoSection<C, S>
where
    C: SpliceCommand,
    S: EncodingState,
{
    state: SpliceInfoState<C>,
    encoded: S,
}

struct SpliceInfoState<C>
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

pub trait EncodingState {}

struct NotEncoded;

impl EncodingState for NotEncoded {}

struct EncodedData {
    section_length: u16,
    splice_command_length: u16,
    splice_command_type: SpliceCommandType,
    descriptor_loop_length: u16,
    crc32: u32,
    final_data: Vec<u8>,
}

impl EncodingState for EncodedData {}

impl<C> SpliceInfoSection<C, NotEncoded>
where
    C: SpliceCommand,
{
    fn new(splice_command: C) -> Self {
        Self {
            state: SpliceInfoState {
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
            },
            encoded: NotEncoded,
        }
    }

    pub fn set_sap_type(&mut self, sap_type: SAPType) {
        self.state.sap_type = sap_type;
    }

    pub fn set_pts_adjustment(&mut self, pts_adjustment: u64) {
        self.state.pts_adjustment = pts_adjustment;
    }

    pub fn set_tier(&mut self, tier: u16) {
        self.state.tier = tier;
    }

    pub fn add_descriptor(&mut self, descriptor: SpliceDescriptor) {
        self.state.descriptors.push(descriptor);
    }

    pub fn remove_descriptor(&mut self, index: usize) {
        self.state.descriptors.remove(index);
    }

    pub fn descriptor_index(&self, descriptor: &SpliceDescriptor) -> Option<usize> {
        self.state.descriptors.iter().position(|d| d == descriptor)
    }

    pub fn get_descriptor_mut(&mut self, index: usize) -> Option<&mut SpliceDescriptor> {
        self.state.descriptors.get_mut(index)
    }
}

impl<C> SpliceInfoSection<C, NotEncoded>
where
    C: SpliceCommand,
{
    pub fn into_encoded(self) -> Result<SpliceInfoSection<C, EncodedData>, CueError> {
        // Write splice command to a temporary buffer
        let mut splice_data = Vec::new();
        self.state.splice_command.write_to(&mut splice_data)?;

        // Write the descriptors to a temporary buffer
        let mut descriptor_data = Vec::new();
        for descriptor in &self.state.descriptors {
            descriptor.write_to(&mut descriptor_data)?;
        }

        // Start writing the final output to a temporary buffer
        let mut data = Vec::new();
        let mut buffer = BitWriter::endian(&mut data, BigEndian);
        buffer.write(8, self.state.table_id)?;
        buffer.write_bit(self.state.section_syntax_indicator)?;
        buffer.write_bit(self.state.private_indicator)?;
        buffer.write(2, self.state.sap_type as u8)?;

        // We know the section length by computing all known fixed size elements from now plus the
        // splice command length and descriptors which are also known by now
        const FIXED_INFO_SIZE_BYTES: usize = (8 + 1 + 6 + 33 + 8 + 12 + 12 + 8 + 16 + 32) / 8;
        let mut section_length =
            (FIXED_INFO_SIZE_BYTES + splice_data.len() + descriptor_data.len()) as u16;
        if self.state.encrypted_packet {
            section_length += 4;
        }
        buffer.write(12, section_length)?;
        buffer.write(8, self.state.protocol_version)?;
        buffer.write_bit(self.state.encrypted_packet)?;
        let encryption_algorithm: u8 = self.state.encryption_algorithm.into();
        buffer.write(6, encryption_algorithm)?;
        buffer.write(33, self.state.pts_adjustment)?;
        buffer.write(8, self.state.cw_index)?;
        buffer.write(12, self.state.tier)?;
        let splice_command_length = splice_data.len() as u16;
        buffer.write(12, splice_command_length)?;
        let splice_command_type = self.state.splice_command.splice_command_type();
        buffer.write(8, u8::from(splice_command_type))?;
        buffer.write_bytes(splice_data.as_slice())?;
        let descriptor_loop_length = descriptor_data.len() as u16;
        buffer.write(16, descriptor_loop_length)?;
        buffer.write_bytes(descriptor_data.as_slice())?;
        buffer.flush()?;

        // Finally, write to out
        let mut final_data = Vec::new();
        let mut buffer = BitWriter::endian(&mut final_data, BigEndian);
        buffer.write_bytes(data.as_slice())?;
        // CRC 32
        if self.state.encrypted_packet {
            // TODO: alignment stuffing here, in case of DES encryption this needs to be 8 bytes aligned
            // encrypted_packet_crc32:
            buffer.write(32, 0)?;
        }
        let crc32 = MPEG_2.checksum(data.as_slice());
        buffer.write(32, crc32)?;
        buffer.flush()?;

        Ok(SpliceInfoSection {
            state: self.state,
            encoded: EncodedData {
                section_length,
                splice_command_length,
                splice_command_type,
                descriptor_loop_length,
                crc32,
                final_data,
            },
        })
    }
}

impl<C> SpliceInfoSection<C, EncodedData>
where
    C: SpliceCommand,
{
    pub fn as_base64(&self) -> Result<String, CueError> {
        Ok(base64::encode(self.encoded.final_data.as_slice()))
    }

    pub fn as_hex(&self) -> Result<String, CueError> {
        Ok(format!(
            "0x{}",
            hex::encode(self.encoded.final_data.as_slice())
        ))
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

impl Display for SAPType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SAPType::Type1 => write!(f, "Type 1"),
            SAPType::Type2 => write!(f, "Type 2"),
            SAPType::Type3 => write!(f, "Type 3"),
            SAPType::NotSpecified => write!(f, "Not Specified"),
        }
    }
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

#[cfg(feature = "serde")]
mod serde_serialization {
    use super::*;
    use serde::ser::{Serialize, SerializeStruct, Serializer};
    use std::fmt::LowerHex;

    impl<C> Serialize for SpliceInfoSection<C, EncodedData>
    where
        C: SpliceCommand + Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            #[inline]
            fn as_hex<T>(value: T) -> String
            where
                T: LowerHex,
            {
                format!("0x{:x}", value)
            }

            let mut state = serializer.serialize_struct("SpliceInfoSection", 17)?;
            state.serialize_field("table_id", &as_hex(self.state.table_id))?;
            state.serialize_field(
                "section_syntax_indicator",
                &self.state.section_syntax_indicator,
            )?;
            state.serialize_field("private_indicator", &self.state.private_indicator)?;
            state.serialize_field("sap_type", &as_hex(self.state.sap_type as u8))?;
            state.serialize_field("section_length", &self.encoded.section_length)?;
            state.serialize_field("protocol_version", &self.state.protocol_version)?;
            state.serialize_field("encrypted_packet", &self.state.encrypted_packet)?;
            state.serialize_field(
                "encryption_algorithm",
                &u8::from(self.state.encryption_algorithm),
            )?;
            state.serialize_field("pts_adjustment", &self.state.pts_adjustment)?;
            state.serialize_field("cw_index", &as_hex(self.state.cw_index))?;
            state.serialize_field("tier", &as_hex(self.state.tier))?;
            state.serialize_field("splice_command_length", &self.encoded.splice_command_length)?;
            state.serialize_field(
                "splice_command_type",
                &u8::from(self.encoded.splice_command_type),
            )?;
            state.serialize_field("splice_command", &self.state.splice_command)?;
            state.serialize_field(
                "descriptor_loop_length",
                &self.encoded.descriptor_loop_length,
            )?;
            state.serialize_field("descriptors", &self.state.descriptors)?;
            state.serialize_field("crc_32", &as_hex(self.encoded.crc32))?;
            state.end()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::SpliceNull;
    use anyhow::Result;
    use assert_json_diff::assert_json_eq;

    #[test]
    fn write_splice_null_as_base64() -> Result<()> {
        let splice = SpliceInfoSection::new(SpliceNull::new());

        assert_eq!(
            splice.into_encoded()?.as_base64()?,
            "/DARAAAAAAAAAP/wAAAAAHpPv/8=".to_string()
        );

        Ok(())
    }

    #[test]
    fn write_splice_null_as_hex() -> Result<()> {
        let splice = SpliceInfoSection::new(SpliceNull::new());

        assert_eq!(
            splice.into_encoded()?.as_hex()?,
            "0xfc301100000000000000fff0000000007a4fbfff".to_string()
        );

        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize_as_json() -> Result<()> {
        let splice = SpliceInfoSection::new(SpliceNull::new());

        assert_json_eq!(
            serde_json::to_value(&splice.into_encoded()?)?,
            serde_json::json!({
                "table_id": "0xfc",
                "section_syntax_indicator": false,
                "private_indicator": false,
                "sap_type": "0x3",
                "section_length": 17,
                "protocol_version": 0,
                "encrypted_packet": false,
                "encryption_algorithm": 0,
                "pts_adjustment": 0,
                "cw_index": "0x0",
                "tier": "0xfff",
                "splice_command_length": 0,
                "splice_command_type": 0,
                "splice_command": {},
                "descriptor_loop_length": 0,
                "descriptors": [],
                "crc_32": "0x7a4fbfff"
            })
        );

        Ok(())
    }
}
