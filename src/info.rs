use crate::commands::{SpliceCommand, SpliceCommandType};
use crate::descriptors::SpliceDescriptor;
use crate::{CueError, TransportPacketWrite};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use crc::{Crc, CRC_32_MPEG_2};
use std::fmt;
use std::fmt::{Display, Formatter};

pub const MPEG_2: Crc<u32> = Crc::<u32>::new(&CRC_32_MPEG_2);

#[derive(Debug, Clone, PartialEq)]
pub struct SpliceInfoSection<C, S>
where
    C: SpliceCommand,
    S: EncodingState,
{
    state: SpliceInfoState<C>,
    encoded: S,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct NotEncoded;

impl EncodingState for NotEncoded {}

#[derive(Debug, Clone, PartialEq)]
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

    pub fn set_cw_index(&mut self, cw_index: u8) {
        self.state.cw_index = cw_index;
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
    pub fn into_encoded(mut self) -> anyhow::Result<SpliceInfoSection<C, EncodedData>> {
        // Write splice command to a temporary buffer
        let mut splice_data = Vec::new();
        let splice_command_length = self.state.splice_command.write_to(&mut splice_data)? as u16;

        // Write the descriptors to a temporary buffer
        let mut descriptor_data = Vec::new();
        let mut descriptor_loop_length = 0;
        for descriptor in &mut self.state.descriptors {
            descriptor_loop_length += descriptor.write_to(&mut descriptor_data)? as u16;
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
        let mut section_length = (FIXED_INFO_SIZE_BYTES
            + splice_command_length as usize
            + descriptor_loop_length as usize) as u16;
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
        buffer.write(12, splice_command_length)?;
        let splice_command_type = self.state.splice_command.splice_command_type();
        buffer.write(8, u8::from(splice_command_type))?;
        buffer.write_bytes(splice_data.as_slice())?;
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
            buffer.write(32, u32::MAX)?;
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
    pub fn to_base64(&self) -> String {
        base64::encode(self.as_bytes())
    }

    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.as_bytes()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.encoded.final_data.as_slice()
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
    use crate::ticks_to_secs;
    use crate::time::format_duration;
    use serde::ser::{Serialize, SerializeStruct, Serializer};
    use std::fmt::LowerHex;
    use std::time::Duration;

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

            let mut state = serializer.serialize_struct("SpliceInfoSection", 19)?;
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
            let pts_adjustment_secs = ticks_to_secs(self.state.pts_adjustment);
            state.serialize_field("pts_adjustment_secs", &pts_adjustment_secs)?;
            state.serialize_field(
                "pts_adjustment_human",
                &format_duration(Duration::from_secs_f64(pts_adjustment_secs)).to_string(),
            )?;
            state.serialize_field("cw_index", &as_hex(self.state.cw_index))?;
            state.serialize_field("tier", &as_hex(self.state.tier))?;
            state.serialize_field("splice_command_length", &self.encoded.splice_command_length)?;
            state.serialize_field(
                "splice_command_type",
                &u8::from(self.encoded.splice_command_type),
            )?;
            state.serialize_field("splice_command_name", &self.encoded.splice_command_type)?;
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
    use crate::commands::*;
    use crate::descriptors::{SegmentationDescriptor, SegmentationType, SegmentationUpid};
    use crate::ClockTimeExt;
    use anyhow::Result;
    use assert_json_diff::assert_json_eq;
    use std::time::Duration;

    #[test]
    fn write_splice_null_as_base64() -> Result<()> {
        let splice = SpliceInfoSection::new(SpliceNull::default());

        assert_eq!(
            splice.into_encoded()?.to_base64(),
            "/DARAAAAAAAAAP/wAAAAAHpPv/8=".to_string()
        );

        Ok(())
    }

    #[test]
    fn write_splice_null_as_hex() -> Result<()> {
        let splice = SpliceInfoSection::new(SpliceNull::default());

        assert_eq!(
            splice.into_encoded()?.to_hex(),
            "0xfc301100000000000000fff0000000007a4fbfff".to_string()
        );

        Ok(())
    }

    fn spec_14_1_example_time_signal() -> Result<SpliceInfoSection<TimeSignal, EncodedData>> {
        let mut splice = SpliceInfoSection::new(TimeSignal::from(0x072bd0050u64));
        splice.set_cw_index(0xff);

        let mut descriptor = SegmentationDescriptor::default();
        descriptor.set_segmentation_event_id(0x4800008e);
        descriptor.set_program_segmentation_flag(true);
        descriptor.set_segmentation_duration_flag(true);
        descriptor.set_no_regional_blackout_flag(true);
        descriptor.set_archive_allowed_flag(true);
        descriptor.set_segmentation_duration(27630000);
        descriptor.set_segmentation_upid(SegmentationUpid::AiringID(0x2ca0a18a));
        descriptor.set_segmentation_type(SegmentationType::ProviderPlacementOpportunityStart);
        descriptor.set_segment_num(2);
        descriptor.set_sub_segment_num(154);
        descriptor.set_sub_segments_expected(201);

        splice.add_descriptor(descriptor.into());

        Ok(splice.into_encoded()?)
    }

    #[test]
    fn compliance_spec_14_1_example_time_signal_as_base64() -> Result<()> {
        assert_eq!(
            spec_14_1_example_time_signal()?.to_base64(),
            // This example was encoded using the threefive Python library
            "/DA2AAAAAAAA///wBQb+cr0AUAAgAh5DVUVJSAAAjn/PAAGlmbAICAAAAAAsoKGKNAIAmsm2waDx"
                .to_string()
        );
        Ok(())
    }

    #[test]
    fn compliance_spec_14_1_example_time_signal_as_hex() -> Result<()> {
        assert_eq!(
            spec_14_1_example_time_signal()?.to_hex(),
            // This example was encoded using the threefive Python library
            "0xfc3036000000000000fffff00506fe72bd00500020021e435545494800008e7fcf0001a599b00808000000002ca0a18a3402009ac9b6c1a0f1".to_string()
        );
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn compliance_spec_14_1_example_time_signal_as_json() -> Result<()> {
        let expected_json: serde_json::Value = serde_json::from_str(
            r#"{
            "table_id": "0xfc",
            "section_syntax_indicator": false,
            "private_indicator": false,
            "sap_type": "0x3",
            "section_length": 54,
            "protocol_version": 0,
            "encrypted_packet": false,
            "encryption_algorithm": 0,
            "pts_adjustment": 0,
            "pts_adjustment_secs": 0.0,
            "pts_adjustment_human": "0s",
            "cw_index": "0xff",
            "tier": "0xfff",
            "splice_command_length": 5,
            "splice_command_type": 6,
            "splice_command_name": "TimeSignal",
            "splice_command": {
                "name": "Time Signal",
                "command_type": 6,
                "command_length": 5,
                "time_specified_flag": true,
                "pts_time": 21388.766756,
                "pts_time_ticks": 1924989008
            },
            "descriptor_loop_length": 32,
            "descriptors": [
                {
                    "name": "Segmentation Descriptor",
                    "splice_descriptor_tag": "0x02",
                    "descriptor_length": 30,
                    "identifier": "CUEI",
                    "segmentation_event_id": "0x4800008e",
                    "segmentation_event_cancel_indicator": false,
                    "program_segmentation_flag": true,
                    "segmentation_duration_flag": true,
                    "delivery_not_restricted_flag": false,
                    "web_delivery_allowed_flag": false,
                    "no_regional_blackout_flag": true,
                    "archive_allowed_flag": true,
                    "device_restrictions": "None",
                    "components": [],
                    "segmentation_duration": 27630000,
                    "segmentation_duration_secs": 307.0,
                    "segmentation_duration_human": "5m 7s",
                    "segmentation_upid_type": "0x08",
                    "segmentation_upid_type_name": "AiringID",
                    "segmentation_upid_length": 8,
                    "segmentation_upid": "0x2ca0a18a",
                    "segmentation_message": "Provider Placement Opportunity Start",
                    "segmentation_type_id": 52,
                    "segment_num": 2,
                    "segments_expected": 0,
                    "sub_segment_num": 154,
                    "sub_segments_expected": 201
                }
            ],
            "crc_32": "0xb6c1a0f1"
        }"#,
        )?;

        assert_json_eq!(
            serde_json::to_value(&spec_14_1_example_time_signal()?)?,
            expected_json
        );

        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize_as_json() -> Result<()> {
        let splice = SpliceInfoSection::new(SpliceNull::default());

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
                "pts_adjustment_secs": 0.0,
                "pts_adjustment_human": "0s",
                "cw_index": "0x0",
                "tier": "0xfff",
                "splice_command_length": 0,
                "splice_command_type": 0,
                "splice_command_name": "SpliceNull",
                "splice_command": {},
                "descriptor_loop_length": 0,
                "descriptors": [],
                "crc_32": "0x7a4fbfff"
            })
        );

        Ok(())
    }
}
