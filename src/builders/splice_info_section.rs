//! Builder for creating SCTE-35 splice information sections.

use crate::types::{SpliceInfoSection, SpliceCommand};
use crate::descriptors::{SegmentationDescriptor, SpliceDescriptor};
use super::error::{BuilderError, BuilderResult};
use super::extensions::SpliceCommandExt;

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

        // Calculate section_length and other derived fields
        let splice_command_length = splice_command.encoded_length();
        let descriptor_loop_length = self.descriptors.iter()
            .map(|d| 2 + d.length() as u16)
            .sum::<u16>();
        let section_length = 11 + splice_command_length + 2 + descriptor_loop_length + 4;

        Ok(SpliceInfoSection {
            table_id: 0xFC,  // Fixed per spec
            section_syntax_indicator: 0,  // Fixed per spec
            private_indicator: 0,  // Fixed per spec
            sap_type: 0x3,  // Fixed per spec (undefined)
            section_length,
            protocol_version: 0,  // Current version
            encrypted_packet: 0,  // Not exposing encryption in builder
            encryption_algorithm: 0,
            pts_adjustment: self.pts_adjustment,
            cw_index: 0,  // Not exposing encryption
            tier: self.tier,
            splice_command_length,
            splice_command_type: (&splice_command).into(),
            splice_command,
            descriptor_loop_length,
            splice_descriptors: self.descriptors,
            alignment_stuffing_bits: Vec::new(),  // Calculated during serialization
            e_crc_32: None,  // Not exposing encryption
            crc_32: 0,  // Calculated during serialization
        })
    }
}

impl Default for SpliceInfoSectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}