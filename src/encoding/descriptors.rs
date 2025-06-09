//! Encoding implementations for SCTE-35 descriptors.

use crate::descriptors::*;
use crate::encoding::{BitWriter, Encodable, EncodingResult};

impl Encodable for SpliceDescriptor {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        match self {
            SpliceDescriptor::Segmentation(desc) => desc.encode(writer),
            SpliceDescriptor::Avail(desc) => desc.encode(writer),
            SpliceDescriptor::Dtmf(desc) => desc.encode(writer),
            SpliceDescriptor::Time(desc) => desc.encode(writer),
            SpliceDescriptor::Audio(desc) => desc.encode(writer),
            SpliceDescriptor::Unknown { tag, length, data } => {
                // splice_descriptor_tag (8 bits)
                writer.write_bits(*tag as u64, 8)?;

                // descriptor_length (8 bits)
                writer.write_bits(*length as u64, 8)?;

                // descriptor data
                writer.write_bytes(data)?;

                Ok(())
            }
        }
    }

    fn encoded_size(&self) -> usize {
        match self {
            SpliceDescriptor::Segmentation(desc) => desc.encoded_size(),
            SpliceDescriptor::Avail(desc) => desc.encoded_size(),
            SpliceDescriptor::Dtmf(desc) => desc.encoded_size(),
            SpliceDescriptor::Time(desc) => desc.encoded_size(),
            SpliceDescriptor::Audio(desc) => desc.encoded_size(),
            SpliceDescriptor::Unknown { data, .. } => 2 + data.len(), // tag + length + data
        }
    }
}

impl Encodable for SegmentationDescriptor {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // splice_descriptor_tag (8 bits) - 0x02 for segmentation descriptor
        writer.write_bits(0x02u64, 8)?;

        // descriptor_length (8 bits) - calculate from fields
        let descriptor_length = self.calculate_descriptor_length();
        writer.write_bits(descriptor_length as u64, 8)?;

        // identifier (32 bits) - 0x43554549 ("CUEI")
        writer.write_bits(0x43554549u64, 32)?;

        // segmentation_event_id (32 bits)
        writer.write_bits(self.segmentation_event_id as u64, 32)?;

        // segmentation_event_cancel_indicator (1 bit)
        writer.write_bits(self.segmentation_event_cancel_indicator as u64, 1)?;

        // Reserved field - there's no segmentation_event_id_compliance_indicator field
        writer.write_bits(1u64, 1)?; // Reserved bit should be 1

        // reserved (6 bits) - should be all 1s
        writer.write_bits(0x3F, 6)?; // 0x3F = 111111 in binary

        if !self.segmentation_event_cancel_indicator {
            // program_segmentation_flag (1 bit)
            writer.write_bits(self.program_segmentation_flag as u64, 1)?;

            // segmentation_duration_flag (1 bit)
            writer.write_bits(self.segmentation_duration_flag as u64, 1)?;

            // delivery_not_restricted_flag (1 bit)
            writer.write_bits(self.delivery_not_restricted_flag as u64, 1)?;

            if !self.delivery_not_restricted_flag {
                // web_delivery_allowed_flag (1 bit)
                let web_flag = self.web_delivery_allowed_flag.unwrap_or(false) as u64;
                writer.write_bits(web_flag, 1)?;

                // no_regional_blackout_flag (1 bit)
                let blackout_flag = self.no_regional_blackout_flag.unwrap_or(false) as u64;
                writer.write_bits(blackout_flag, 1)?;

                // archive_allowed_flag (1 bit)
                let archive_flag = self.archive_allowed_flag.unwrap_or(false) as u64;
                writer.write_bits(archive_flag, 1)?;

                // device_restrictions (2 bits)
                let restrictions = self.device_restrictions.unwrap_or(0) as u64;
                writer.write_bits(restrictions, 2)?;
            } else {
                // reserved (5 bits) - should be all 1s
                writer.write_bits(0x1F, 5)?; // 0x1F = 11111 in binary
            }

            // Component loop if program_segmentation_flag == false
            if !self.program_segmentation_flag {
                // For now, assume no components since they're not in the struct
                // component_count (8 bits)
                writer.write_bits(0u64, 8)?; // This is data, not reserved bits
            }

            // segmentation_duration if segmentation_duration_flag == true
            #[allow(clippy::collapsible_if)]
            if self.segmentation_duration_flag {
                if let Some(duration) = self.segmentation_duration {
                    writer.write_bits(duration & 0xFFFFFFFFFF, 40)?; // 40 bits
                }
            }
        }

        // segmentation_upid_type (8 bits)
        let upid_type_value: u8 = self.segmentation_upid_type.into();
        writer.write_bits(upid_type_value as u64, 8)?;

        // segmentation_upid_length (8 bits)
        writer.write_bits(self.segmentation_upid_length as u64, 8)?;

        // segmentation_upid (variable length)
        writer.write_bytes(&self.segmentation_upid)?;

        // segmentation_type_id (8 bits)
        writer.write_bits(self.segmentation_type_id as u64, 8)?;

        // segment_num (8 bits)
        writer.write_bits(self.segment_num as u64, 8)?;

        // segments_expected (8 bits)
        writer.write_bits(self.segments_expected as u64, 8)?;

        // Sub-segment fields for specific segmentation types that support sub-segments
        // 0x34 (Provider Placement Opportunity Start) does NOT have sub-segment fields
        if matches!(
            self.segmentation_type_id,
            0x30 | 0x32 | 0x36 | 0x38 | 0x3A | 0x44 | 0x46
        ) {
            if let Some(sub_segment_num) = self.sub_segment_num {
                writer.write_bits(sub_segment_num as u64, 8)?;
            }
            if let Some(sub_segments_expected) = self.sub_segments_expected {
                writer.write_bits(sub_segments_expected as u64, 8)?;
            }
        }

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        let mut size = 2 + 4 + 4 + 1; // tag + length + identifier + event_id + flags

        if !self.segmentation_event_cancel_indicator {
            size += 1; // flags byte

            // Component loop
            if !self.program_segmentation_flag {
                size += 1; // component_count (assuming 0 components for now)
            }

            // Duration
            if self.segmentation_duration_flag {
                size += 5; // 40 bits = 5 bytes
            }
        }

        size += 2; // upid_type + upid_length
        size += self.segmentation_upid.len(); // upid data
        size += 3; // type_id + segment_num + segments_expected

        // Sub-segment fields - 0x34 (Provider Placement Opportunity Start) does NOT have sub-segment fields
        if matches!(
            self.segmentation_type_id,
            0x30 | 0x32 | 0x36 | 0x38 | 0x3A | 0x44 | 0x46
        ) {
            size += 2; // sub_segment_num + sub_segments_expected
        }

        size
    }
}

impl SegmentationDescriptor {
    fn calculate_descriptor_length(&self) -> usize {
        // Calculate length excluding tag and length field itself
        self.encoded_size() - 2
    }
}

// Placeholder implementations for other descriptor types
impl Encodable for AvailDescriptor {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // splice_descriptor_tag (8 bits)
        writer.write_bits(0x00u64, 8)?;

        // descriptor_length (8 bits) - 4 bytes for identifier + provider_avail_id length
        let length = 4 + self.provider_avail_id.len();
        writer.write_bits(length as u64, 8)?;

        // identifier (32 bits)
        writer.write_bits(self.identifier as u64, 32)?;

        // provider_avail_id (variable length)
        writer.write_bytes(&self.provider_avail_id)?;

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        2 + 4 + self.provider_avail_id.len() // tag + length + identifier + provider_avail_id
    }
}

impl Encodable for DtmfDescriptor {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // splice_descriptor_tag (8 bits)
        writer.write_bits(0x01u64, 8)?;

        // descriptor_length (8 bits)
        writer.write_bits(4u64, 8)?;

        // identifier (32 bits)
        writer.write_bits(self.identifier as u64, 32)?;

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        6 // tag + length + identifier
    }
}

impl Encodable for TimeDescriptor {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // splice_descriptor_tag (8 bits)
        writer.write_bits(0x03u64, 8)?;

        // descriptor_length (8 bits)
        writer.write_bits(
            (4 + self.tai_seconds.len() + self.tai_ns.len() + self.utc_offset.len()) as u64,
            8,
        )?;

        // identifier (32 bits)
        writer.write_bits(self.identifier as u64, 32)?;

        // tai_seconds
        writer.write_bytes(&self.tai_seconds)?;

        // tai_ns
        writer.write_bytes(&self.tai_ns)?;

        // utc_offset
        writer.write_bytes(&self.utc_offset)?;

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        2 + 4 + self.tai_seconds.len() + self.tai_ns.len() + self.utc_offset.len()
    }
}

impl Encodable for AudioDescriptor {
    fn encode(&self, writer: &mut BitWriter) -> EncodingResult<()> {
        // splice_descriptor_tag (8 bits)
        writer.write_bits(0x04u64, 8)?;

        // descriptor_length (8 bits)
        writer.write_bits((4 + self.audio_components.len()) as u64, 8)?;

        // identifier (32 bits)
        writer.write_bits(self.identifier as u64, 32)?;

        // audio_components
        writer.write_bytes(&self.audio_components)?;

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        2 + 4 + self.audio_components.len()
    }
}
