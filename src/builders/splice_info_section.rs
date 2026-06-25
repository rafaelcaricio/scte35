//! Builder for creating SCTE-35 splice information sections.

use super::error::{BuilderError, BuilderResult};
use crate::descriptors::{SegmentationDescriptor, SpliceDescriptor};
use crate::types::{SpliceCommand, SpliceInfoSection};

/// Builder for creating a complete SCTE-35 splice information section.
///
/// This is the top-level builder that creates the complete SCTE-35 message
/// containing a splice command and optional descriptors.
#[derive(Debug)]
pub struct SpliceInfoSectionBuilder {
    pts_adjustment: u64,
    tier: u16,
    cw_index: u8,
    splice_command: Option<SpliceCommand>,
    descriptors: Vec<SpliceDescriptor>,
}

impl SpliceInfoSectionBuilder {
    /// Create a new splice info section builder with default values.
    pub fn new() -> Self {
        Self {
            pts_adjustment: 0,
            tier: 0xFFF,    // Default "all tiers"
            cw_index: 0x00, // No control word
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

    /// Set the control word index (8-bit).
    ///
    /// Specifies the control word index for conditional access systems.
    /// 0x00 = no control word, 0xFF = no control word (all 1s)
    pub fn cw_index(mut self, cw_index: u8) -> Self {
        self.cw_index = cw_index;
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
        self.descriptors
            .push(SpliceDescriptor::Segmentation(descriptor));
        self
    }

    /// Build the final splice info section.
    ///
    /// # Errors
    ///
    /// Returns an error if no splice command has been set.
    pub fn build(self) -> BuilderResult<SpliceInfoSection> {
        let splice_command = self
            .splice_command
            .ok_or(BuilderError::MissingRequiredField("splice_command"))?;
        let splice_command_type = splice_command.command_type();

        // Build the section with proper defaults. Length fields are derived
        // during encoding and are intentionally not stored in the API model.
        let section = SpliceInfoSection {
            table_id: 0xFC,              // Fixed per spec
            section_syntax_indicator: 0, // Fixed per spec
            private_indicator: 0,        // Fixed per spec
            sap_type: 0x3,               // Fixed per spec (undefined)
            protocol_version: 0,         // Current version
            encrypted_packet: 0,         // Not encrypted
            encryption_algorithm: 0,     // No encryption
            pts_adjustment: self.pts_adjustment,
            cw_index: self.cw_index,
            tier: self.tier,
            splice_command_type,
            splice_command,
            splice_descriptors: self.descriptors,
            alignment_stuffing_bits: Vec::new(), // No stuffing by default
            e_crc_32: None,                      // Not encrypted
            crc_32: 0,                           // Will be calculated during encoding
        };

        Ok(section)
    }
}

impl Default for SpliceInfoSectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
