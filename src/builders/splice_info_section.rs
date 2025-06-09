//! Builder for creating SCTE-35 splice information sections.

use crate::types::{SpliceInfoSection, SpliceCommand};
use crate::descriptors::{SegmentationDescriptor, SpliceDescriptor};
use super::error::{BuilderError, BuilderResult};

/// Builder for creating a complete SCTE-35 splice information section.
///
/// This is the top-level builder that creates the complete SCTE-35 message
/// containing a splice command and optional descriptors.
#[derive(Debug)]
pub struct SpliceInfoSectionBuilder {
    pts_adjustment: u64,
    tier: u16,
    splice_command: Option<SpliceCommand>,
    descriptors: Vec<SpliceDescriptor>,
}

impl SpliceInfoSectionBuilder {
    /// Create a new splice info section builder with default values.
    pub fn new() -> Self {
        Self {
            pts_adjustment: 0,
            tier: 0xFFF, // Default "all tiers"
            splice_command: None,
            descriptors: Vec::new(),
        }
    }

    /// Set the PTS adjustment value (33-bit).
    ///
    /// This value is added to all PTS times in the message to adjust for
    /// timing differences between encoding and transmission.
    pub fn pts_adjustment(mut self, pts_adjustment: u64) -> Self {
        self.pts_adjustment = pts_adjustment & 0x1_FFFF_FFFF; // 33-bit value
        self
    }

    /// Set the tier value (12-bit).
    ///
    /// Specifies which tier this message applies to. 0xFFF means all tiers.
    pub fn tier(mut self, tier: u16) -> Self {
        self.tier = tier & 0xFFF; // 12-bit value
        self
    }

    /// Set the splice command directly.
    pub fn splice_command(mut self, command: SpliceCommand) -> Self {
        self.splice_command = Some(command);
        self
    }

    /// Set a splice null command.
    pub fn splice_null(mut self) -> Self {
        self.splice_command = Some(SpliceCommand::SpliceNull);
        self
    }

    /// Set a splice insert command.
    pub fn splice_insert(mut self, insert: crate::types::SpliceInsert) -> Self {
        self.splice_command = Some(SpliceCommand::SpliceInsert(insert));
        self
    }

    /// Set a time signal command.
    pub fn time_signal(mut self, time_signal: crate::types::TimeSignal) -> Self {
        self.splice_command = Some(SpliceCommand::TimeSignal(time_signal));
        self
    }

    /// Add a descriptor to the message.
    pub fn add_descriptor(mut self, descriptor: SpliceDescriptor) -> Self {
        self.descriptors.push(descriptor);
        self
    }

    /// Add a segmentation descriptor to the message.
    pub fn add_segmentation_descriptor(mut self, descriptor: SegmentationDescriptor) -> Self {
        self.descriptors.push(SpliceDescriptor::Segmentation(descriptor));
        self
    }

    /// Build the final splice info section.
    ///
    /// # Errors
    ///
    /// Returns an error if no splice command has been set.
    pub fn build(self) -> BuilderResult<SpliceInfoSection> {
        let splice_command = self.splice_command
            .ok_or(BuilderError::MissingRequiredField("splice_command"))?;

        // Get the actual command type
        let splice_command_type: u8 = (&splice_command).into();
        
        // Import the Encodable trait to get access to encoded sizes
        use crate::encoding::Encodable;
        
        // Calculate descriptor_loop_length correctly
        let mut descriptor_loop_length = 0u16;
        for descriptor in &self.descriptors {
            // Each descriptor contributes its full encoded size
            descriptor_loop_length += descriptor.encoded_size() as u16;
        }

        // Build the section with proper defaults
        let mut section = SpliceInfoSection {
            table_id: 0xFC,  // Fixed per spec
            section_syntax_indicator: 0,  // Fixed per spec  
            private_indicator: 0,  // Fixed per spec
            sap_type: 0x3,  // Fixed per spec (undefined)
            section_length: 0,  // Will be calculated during encoding
            protocol_version: 0,  // Current version
            encrypted_packet: 0,  // Not encrypted
            encryption_algorithm: 0,  // No encryption
            pts_adjustment: self.pts_adjustment,
            cw_index: 0xFF,  // No control word (all 1s)
            tier: self.tier,
            splice_command_length: 0,  // Will be calculated during encoding
            splice_command_type,
            splice_command,
            descriptor_loop_length: 0,  // Will be calculated during encoding
            splice_descriptors: self.descriptors,
            alignment_stuffing_bits: Vec::new(),  // No stuffing by default
            e_crc_32: None,  // Not encrypted
            crc_32: 0,  // Will be calculated during encoding
        };
        
        // Calculate the actual lengths now that we have the full structure
        section.splice_command_length = section.splice_command.encoded_size() as u16;
        section.descriptor_loop_length = descriptor_loop_length;
        
        // Section length is the total size minus the first 3 bytes
        // (table_id + section_syntax_indicator/private_indicator/sap_type + section_length itself)
        section.section_length = (section.encoded_size() - 3) as u16;
        
        Ok(section)
    }
}

impl Default for SpliceInfoSectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}