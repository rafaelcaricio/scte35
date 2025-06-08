//! Core SCTE-35 data structures and types.
//!
//! This module contains the main structures representing SCTE-35 messages,
//! commands, and related components.

use crate::descriptors::SpliceDescriptor;
use crate::time::{BreakDuration, DateTime, SpliceTime};

/// Represents a complete SCTE-35 splice information section.
///
/// This is the top-level structure that contains all the information from an SCTE-35 message,
/// including the header fields, splice command, descriptors, and CRC.
///
/// # Fields
///
/// The structure follows the SCTE-35 specification layout:
/// - Header fields (table_id, section_length, etc.)
/// - Splice command data
/// - Optional descriptors
/// - CRC for data integrity
#[derive(Debug)]
pub struct SpliceInfoSection {
    /// Table identifier, should be 0xFC for SCTE-35
    pub table_id: u8,
    /// Section syntax indicator (0 for MPEG short section)
    pub section_syntax_indicator: u8,
    /// Private indicator (0 for not private)
    pub private_indicator: u8,
    /// SAP (Stream Access Point) type
    pub sap_type: u8,
    /// Length of the section in bytes
    pub section_length: u16,
    /// SCTE-35 protocol version
    pub protocol_version: u8,
    /// Encryption packet flag (0 for unencrypted)
    pub encrypted_packet: u8,
    /// Encryption algorithm used
    pub encryption_algorithm: u8,
    /// PTS adjustment value in 90kHz ticks
    pub pts_adjustment: u64,
    /// Control word index for encryption
    pub cw_index: u8,
    /// Tier value for authorization
    pub tier: u16,
    /// Length of the splice command in bytes
    pub splice_command_length: u16,
    /// Type of splice command (0x00-0xFF)
    pub splice_command_type: u8,
    /// The actual splice command data
    pub splice_command: SpliceCommand,
    /// Length of descriptor loop in bytes
    pub descriptor_loop_length: u16,
    /// List of splice descriptors
    pub splice_descriptors: Vec<SpliceDescriptor>,
    /// Alignment stuffing bits for byte alignment
    pub alignment_stuffing_bits: Vec<u8>,
    /// Encrypted CRC-32 (present when encrypted_packet = 1)
    pub e_crc_32: Option<u32>,
    /// CRC-32 checksum of the section
    pub crc_32: u32,
}

/// Represents the different types of splice commands defined in SCTE-35.
///
/// Each variant contains the specific data structure for that command type.
/// The command type determines how the splice operation should be performed.
#[derive(Debug)]
pub enum SpliceCommand {
    /// Null command (0x00) - No operation
    SpliceNull,
    /// Splice schedule command (0x04) - Scheduled splice events
    SpliceSchedule(SpliceSchedule),
    /// Splice insert command (0x05) - Ad insertion points
    SpliceInsert(SpliceInsert),
    /// Time signal command (0x06) - Time synchronization
    TimeSignal(TimeSignal),
    /// Bandwidth reservation command (0x07) - Bandwidth allocation
    BandwidthReservation(BandwidthReservation),
    /// Private command (0xFF) - Custom/proprietary commands
    PrivateCommand(PrivateCommand),
    /// Unknown command type
    Unknown,
}

/// Represents a splice null command.
///
/// This command indicates no splice operation should be performed.
/// It's used as a placeholder or to clear previous splice commands.
#[derive(Debug)]
pub struct SpliceNull {}

/// Represents a splice schedule command (0x04).
///
/// This command schedules splice events to occur at specific times in the future.
/// It allows for pre-scheduling of ad insertion points or other splice operations.
#[derive(Debug)]
pub struct SpliceSchedule {
    /// Unique identifier for this splice event
    pub splice_event_id: u32,
    /// Indicates if the splice event is being cancelled (1 = cancel, 0 = proceed)
    pub splice_event_cancel_indicator: u8,
    /// Reserved bits for future use
    pub reserved: u8,
    /// Indicates whether the splice is going out of or returning to the network (1 = out, 0 = in)
    pub out_of_network_indicator: u8,
    /// Indicates whether a duration is specified (1 = duration present, 0 = no duration)
    pub duration_flag: u8,
    /// Duration of the splice in 90kHz ticks (present when duration_flag = 1)
    pub splice_duration: Option<u32>,
    /// Scheduled time for the splice to occur (present when duration_flag = 0)
    pub scheduled_splice_time: Option<DateTime>,
    /// Unique identifier for the program
    pub unique_program_id: u16,
    /// Number of components in the component list
    pub num_splice: u8,
    /// List of component-specific splice information
    pub component_list: Vec<ComponentSplice>,
}

/// Represents a splice insert command (0x05).
///
/// This is the most commonly used splice command for ad insertion.
/// It signals the start and end of commercial breaks or other content substitutions.
#[derive(Debug)]
pub struct SpliceInsert {
    /// Unique identifier for this splice event
    pub splice_event_id: u32,
    /// Indicates if the splice event is being cancelled (1 = cancel, 0 = proceed)
    pub splice_event_cancel_indicator: u8,
    /// Reserved bits for future use
    pub reserved: u8,
    /// Indicates whether the splice is going out of or returning to the network (1 = out, 0 = in)
    pub out_of_network_indicator: u8,
    /// Indicates if this is a program-level splice (1) or component-level splice (0)
    pub program_splice_flag: u8,
    /// Indicates whether a break duration is specified (1 = duration present, 0 = no duration)
    pub duration_flag: u8,
    /// Indicates if the splice should happen immediately (1 = immediate, 0 = at specified time)
    pub splice_immediate_flag: u8,
    /// Additional reserved bits
    pub reserved2: u8,
    /// Presentation timestamp when the splice should occur (present when program_splice_flag = 1 and splice_immediate_flag = 0)
    pub splice_time: Option<SpliceTime>,
    /// Number of components in the component list (present when program_splice_flag = 0)
    pub component_count: u8,
    /// List of component-specific splice times (present when program_splice_flag = 0)
    pub components: Vec<SpliceInsertComponent>,
    /// Duration of the commercial break (present when duration_flag = 1)
    pub break_duration: Option<BreakDuration>,
    /// Unique identifier for the program
    pub unique_program_id: u16,
    /// Avail number for this splice event
    pub avail_num: u8,
    /// Expected number of avails in this break
    pub avails_expected: u8,
}

/// Represents a time signal command (0x06).
///
/// This command provides time synchronization information and is often used
/// with segmentation descriptors to indicate various types of content boundaries.
#[derive(Debug)]
pub struct TimeSignal {
    /// The presentation timestamp for this time signal
    pub splice_time: SpliceTime,
}

/// Represents a bandwidth reservation command (0x07).
///
/// This command is used to reserve bandwidth for future use,
/// typically in cable systems for managing network capacity.
#[derive(Debug)]
pub struct BandwidthReservation {
    /// Reserved bits for future use
    pub reserved: u8,
    /// Bandwidth reservation value in kilobits per second
    pub dwbw_reservation: u32,
}

/// Represents a private command (0xFF).
///
/// This command allows for custom, proprietary splice operations
/// that are not defined in the standard SCTE-35 specification.
#[derive(Debug)]
pub struct PrivateCommand {
    /// Identifier for the private command type
    pub private_command_id: u16,
    /// Length of the private command data in bytes
    pub private_command_length: u8,
    /// Raw bytes containing the private command data
    pub private_bytes: Vec<u8>,
}

/// Represents component-specific splice information for splice schedule commands.
///
/// This structure contains timing and mode information for individual components
/// when performing component-level splicing operations.
#[derive(Debug)]
pub struct ComponentSplice {
    /// Identifier for the specific component (audio/video track)
    pub component_tag: u8,
    /// Reserved bits for future use
    pub reserved: u8,
    /// Indicates the splice mode for this component
    pub splice_mode_indicator: u8,
    /// Indicates whether a duration is specified (1 = duration present, 0 = scheduled time present)
    pub duration_flag: u8,
    /// Duration of the splice for this component in 90kHz ticks (present when duration_flag = 1)
    pub splice_duration: Option<u32>,
    /// Scheduled time for the splice to occur (present when duration_flag = 0)
    pub scheduled_splice_time: Option<DateTime>,
}

/// Represents component-specific timing information for splice insert commands.
///
/// This structure contains the splice time for individual components
/// when performing component-level splice insert operations.
#[derive(Debug)]
pub struct SpliceInsertComponent {
    /// Identifier for the specific component (audio/video track)
    pub component_tag: u8,
    /// Presentation timestamp when this component should splice (present when splice_immediate_flag = 0)
    pub splice_time: Option<SpliceTime>,
}

/// Represents the different types of segmentation as defined in SCTE-35.
///
/// These values indicate the type of content segment boundary being signaled.
/// They provide semantic meaning to segmentation descriptors, allowing systems
/// to understand what type of content transition is occurring.
///
/// # Usage
///
/// This enum is typically used with segmentation descriptors to provide
/// human-readable context for splice operations:
///
/// ```rust
/// use scte35_parsing::SegmentationType;
///
/// let seg_type = SegmentationType::ProviderAdvertisementStart;
/// println!("Segmentation type: {:?} (ID: 0x{:02X})", seg_type, seg_type.id());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SegmentationType {
    /// Not indicated (0x00) - No specific segmentation type
    NotIndicated,
    /// Content identification (0x01) - Identifies content for tracking
    ContentIdentification,
    /// Program start (0x10) - Beginning of a program
    ProgramStart,
    /// Program end (0x11) - End of a program
    ProgramEnd,
    /// Program early termination (0x12) - Program ended before scheduled time
    ProgramEarlyTermination,
    /// Program breakaway (0x13) - Program interrupted for local content
    ProgramBreakaway,
    /// Program resumption (0x14) - Return to program after breakaway
    ProgramResumption,
    /// Program runover planned (0x15) - Program extending beyond scheduled time
    ProgramRunoverPlanned,
    /// Program runover unplanned (0x16) - Unexpected program extension
    ProgramRunoverUnplanned,
    /// Program overlap start (0x17) - Beginning of overlapping program content
    ProgramOverlapStart,
    /// Program blackout override (0x18) - Override blackout restrictions
    ProgramBlackoutOverride,
    /// Program join (0x19) - Joining program already in progress
    ProgramJoin,
    /// Chapter start (0x20) - Beginning of a chapter or segment
    ChapterStart,
    /// Chapter end (0x21) - End of a chapter or segment
    ChapterEnd,
    /// Break start (0x22) - Beginning of a break period
    BreakStart,
    /// Break end (0x23) - End of a break period
    BreakEnd,
    /// Opening credit start (0x24) - Deprecated, use content identification
    OpeningCreditStartDeprecated,
    /// Opening credit end (0x25) - Deprecated, use content identification
    OpeningCreditEndDeprecated,
    /// Closing credit start (0x26) - Deprecated, use content identification
    ClosingCreditStartDeprecated,
    /// Closing credit end (0x27) - Deprecated, use content identification
    ClosingCreditEndDeprecated,
    /// Provider advertisement start (0x30) - Beginning of provider ad
    ProviderAdvertisementStart,
    /// Provider advertisement end (0x31) - End of provider ad
    ProviderAdvertisementEnd,
    /// Distributor advertisement start (0x32) - Beginning of distributor ad
    DistributorAdvertisementStart,
    /// Distributor advertisement end (0x33) - End of distributor ad
    DistributorAdvertisementEnd,
    /// Provider placement opportunity start (0x34) - Beginning of provider placement
    ProviderPlacementOpportunityStart,
    /// Provider placement opportunity end (0x35) - End of provider placement
    ProviderPlacementOpportunityEnd,
    /// Distributor placement opportunity start (0x36) - Beginning of distributor placement
    DistributorPlacementOpportunityStart,
    /// Distributor placement opportunity end (0x37) - End of distributor placement
    DistributorPlacementOpportunityEnd,
    /// Provider overlay placement opportunity start (0x38) - Beginning of provider overlay
    ProviderOverlayPlacementOpportunityStart,
    /// Provider overlay placement opportunity end (0x39) - End of provider overlay
    ProviderOverlayPlacementOpportunityEnd,
    /// Distributor overlay placement opportunity start (0x3A) - Beginning of distributor overlay
    DistributorOverlayPlacementOpportunityStart,
    /// Distributor overlay placement opportunity end (0x3B) - End of distributor overlay
    DistributorOverlayPlacementOpportunityEnd,
    /// Provider promo start (0x3C) - Beginning of provider promotional content
    ProviderPromoStart,
    /// Provider promo end (0x3D) - End of provider promotional content
    ProviderPromoEnd,
    /// Distributor promo start (0x3E) - Beginning of distributor promotional content
    DistributorPromoStart,
    /// Distributor promo end (0x3F) - End of distributor promotional content
    DistributorPromoEnd,
    /// Unscheduled event start (0x40) - Beginning of unscheduled content
    UnscheduledEventStart,
    /// Unscheduled event end (0x41) - End of unscheduled content
    UnscheduledEventEnd,
    /// Alternate content opportunity start (0x42) - Beginning of alternate content
    AlternateContentOpportunityStart,
    /// Alternate content opportunity end (0x43) - End of alternate content
    AlternateContentOpportunityEnd,
    /// Provider ad block start (0x44) - Beginning of provider ad block
    ProviderAdBlockStart,
    /// Provider ad block end (0x45) - End of provider ad block
    ProviderAdBlockEnd,
    /// Distributor ad block start (0x46) - Beginning of distributor ad block
    DistributorAdBlockStart,
    /// Distributor ad block end (0x47) - End of distributor ad block
    DistributorAdBlockEnd,
    /// Network start (0x50) - Beginning of network content
    NetworkStart,
    /// Network end (0x51) - End of network content
    NetworkEnd,
}

impl Default for SegmentationType {
    fn default() -> Self {
        SegmentationType::NotIndicated
    }
}

impl SegmentationType {
    /// Returns the numeric identifier for this segmentation type.
    ///
    /// These IDs correspond to the values defined in the SCTE-35 specification
    /// for segmentation descriptor types.
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35_parsing::SegmentationType;
    ///
    /// assert_eq!(SegmentationType::ProviderAdvertisementStart.id(), 0x30);
    /// assert_eq!(SegmentationType::ProgramStart.id(), 0x10);
    /// ```
    pub fn id(&self) -> u8 {
        use SegmentationType::*;
        match self {
            NotIndicated => 0x00,
            ContentIdentification => 0x01,
            ProgramStart => 0x10,
            ProgramEnd => 0x11,
            ProgramEarlyTermination => 0x12,
            ProgramBreakaway => 0x13,
            ProgramResumption => 0x14,
            ProgramRunoverPlanned => 0x15,
            ProgramRunoverUnplanned => 0x16,
            ProgramOverlapStart => 0x17,
            ProgramBlackoutOverride => 0x18,
            ProgramJoin => 0x19,
            ChapterStart => 0x20,
            ChapterEnd => 0x21,
            BreakStart => 0x22,
            BreakEnd => 0x23,
            OpeningCreditStartDeprecated => 0x24,
            OpeningCreditEndDeprecated => 0x25,
            ClosingCreditStartDeprecated => 0x26,
            ClosingCreditEndDeprecated => 0x27,
            ProviderAdvertisementStart => 0x30,
            ProviderAdvertisementEnd => 0x31,
            DistributorAdvertisementStart => 0x32,
            DistributorAdvertisementEnd => 0x33,
            ProviderPlacementOpportunityStart => 0x34,
            ProviderPlacementOpportunityEnd => 0x35,
            DistributorPlacementOpportunityStart => 0x36,
            DistributorPlacementOpportunityEnd => 0x37,
            ProviderOverlayPlacementOpportunityStart => 0x38,
            ProviderOverlayPlacementOpportunityEnd => 0x39,
            DistributorOverlayPlacementOpportunityStart => 0x3A,
            DistributorOverlayPlacementOpportunityEnd => 0x3B,
            ProviderPromoStart => 0x3C,
            ProviderPromoEnd => 0x3D,
            DistributorPromoStart => 0x3E,
            DistributorPromoEnd => 0x3F,
            UnscheduledEventStart => 0x40,
            UnscheduledEventEnd => 0x41,
            AlternateContentOpportunityStart => 0x42,
            AlternateContentOpportunityEnd => 0x43,
            ProviderAdBlockStart => 0x44,
            ProviderAdBlockEnd => 0x45,
            DistributorAdBlockStart => 0x46,
            DistributorAdBlockEnd => 0x47,
            NetworkStart => 0x50,
            NetworkEnd => 0x51,
        }
    }

    /// Converts a numeric segmentation type ID to the corresponding enum variant.
    ///
    /// This method is useful for parsing segmentation descriptors from SCTE-35 messages
    /// and converting the raw numeric values into typed enum variants.
    ///
    /// # Arguments
    ///
    /// * `id` - The numeric segmentation type identifier (0x00-0xFF)
    ///
    /// # Returns
    ///
    /// The corresponding `SegmentationType` variant, or `NotIndicated` for unknown values.
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35_parsing::SegmentationType;
    ///
    /// assert_eq!(SegmentationType::from_id(0x30), SegmentationType::ProviderAdvertisementStart);
    /// assert_eq!(SegmentationType::from_id(0x10), SegmentationType::ProgramStart);
    /// assert_eq!(SegmentationType::from_id(0xFF), SegmentationType::NotIndicated); // Unknown value
    /// ```
    pub fn from_id(id: u8) -> Self {
        use SegmentationType::*;
        match id {
            0x00 => NotIndicated,
            0x01 => ContentIdentification,
            0x10 => ProgramStart,
            0x11 => ProgramEnd,
            0x12 => ProgramEarlyTermination,
            0x13 => ProgramBreakaway,
            0x14 => ProgramResumption,
            0x15 => ProgramRunoverPlanned,
            0x16 => ProgramRunoverUnplanned,
            0x17 => ProgramOverlapStart,
            0x18 => ProgramBlackoutOverride,
            0x19 => ProgramJoin,
            0x20 => ChapterStart,
            0x21 => ChapterEnd,
            0x22 => BreakStart,
            0x23 => BreakEnd,
            0x24 => OpeningCreditStartDeprecated,
            0x25 => OpeningCreditEndDeprecated,
            0x26 => ClosingCreditStartDeprecated,
            0x27 => ClosingCreditEndDeprecated,
            0x30 => ProviderAdvertisementStart,
            0x31 => ProviderAdvertisementEnd,
            0x32 => DistributorAdvertisementStart,
            0x33 => DistributorAdvertisementEnd,
            0x34 => ProviderPlacementOpportunityStart,
            0x35 => ProviderPlacementOpportunityEnd,
            0x36 => DistributorPlacementOpportunityStart,
            0x37 => DistributorPlacementOpportunityEnd,
            0x38 => ProviderOverlayPlacementOpportunityStart,
            0x39 => ProviderOverlayPlacementOpportunityEnd,
            0x3A => DistributorOverlayPlacementOpportunityStart,
            0x3B => DistributorOverlayPlacementOpportunityEnd,
            0x3C => ProviderPromoStart,
            0x3D => ProviderPromoEnd,
            0x3E => DistributorPromoStart,
            0x3F => DistributorPromoEnd,
            0x40 => UnscheduledEventStart,
            0x41 => UnscheduledEventEnd,
            0x42 => AlternateContentOpportunityStart,
            0x43 => AlternateContentOpportunityEnd,
            0x44 => ProviderAdBlockStart,
            0x45 => ProviderAdBlockEnd,
            0x46 => DistributorAdBlockStart,
            0x47 => DistributorAdBlockEnd,
            0x50 => NetworkStart,
            0x51 => NetworkEnd,
            _ => NotIndicated, // Default for unknown values
        }
    }

    /// Returns a human-readable description of the segmentation type.
    ///
    /// This method provides descriptive text for each segmentation type that can be
    /// used in user interfaces or logging to make SCTE-35 data more understandable.
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35_parsing::SegmentationType;
    ///
    /// let seg_type = SegmentationType::ProviderAdvertisementStart;
    /// println!("{}", seg_type.description()); // "Provider Advertisement Start"
    /// ```
    pub fn description(&self) -> &'static str {
        use SegmentationType::*;
        match self {
            NotIndicated => "Not Indicated",
            ContentIdentification => "Content Identification",
            ProgramStart => "Program Start",
            ProgramEnd => "Program End",
            ProgramEarlyTermination => "Program Early Termination",
            ProgramBreakaway => "Program Breakaway",
            ProgramResumption => "Program Resumption",
            ProgramRunoverPlanned => "Program Runover Planned",
            ProgramRunoverUnplanned => "Program Runover Unplanned",
            ProgramOverlapStart => "Program Overlap Start",
            ProgramBlackoutOverride => "Program Blackout Override",
            ProgramJoin => "Program Join",
            ChapterStart => "Chapter Start",
            ChapterEnd => "Chapter End",
            BreakStart => "Break Start",
            BreakEnd => "Break End",
            OpeningCreditStartDeprecated => "Opening Credit Start (Deprecated)",
            OpeningCreditEndDeprecated => "Opening Credit End (Deprecated)",
            ClosingCreditStartDeprecated => "Closing Credit Start (Deprecated)",
            ClosingCreditEndDeprecated => "Closing Credit End (Deprecated)",
            ProviderAdvertisementStart => "Provider Advertisement Start",
            ProviderAdvertisementEnd => "Provider Advertisement End",
            DistributorAdvertisementStart => "Distributor Advertisement Start",
            DistributorAdvertisementEnd => "Distributor Advertisement End",
            ProviderPlacementOpportunityStart => "Provider Placement Opportunity Start",
            ProviderPlacementOpportunityEnd => "Provider Placement Opportunity End",
            DistributorPlacementOpportunityStart => "Distributor Placement Opportunity Start",
            DistributorPlacementOpportunityEnd => "Distributor Placement Opportunity End",
            ProviderOverlayPlacementOpportunityStart => {
                "Provider Overlay Placement Opportunity Start"
            }
            ProviderOverlayPlacementOpportunityEnd => "Provider Overlay Placement Opportunity End",
            DistributorOverlayPlacementOpportunityStart => {
                "Distributor Overlay Placement Opportunity Start"
            }
            DistributorOverlayPlacementOpportunityEnd => {
                "Distributor Overlay Placement Opportunity End"
            }
            ProviderPromoStart => "Provider Promo Start",
            ProviderPromoEnd => "Provider Promo End",
            DistributorPromoStart => "Distributor Promo Start",
            DistributorPromoEnd => "Distributor Promo End",
            UnscheduledEventStart => "Unscheduled Event Start",
            UnscheduledEventEnd => "Unscheduled Event End",
            AlternateContentOpportunityStart => "Alternate Content Opportunity Start",
            AlternateContentOpportunityEnd => "Alternate Content Opportunity End",
            ProviderAdBlockStart => "Provider Ad Block Start",
            ProviderAdBlockEnd => "Provider Ad Block End",
            DistributorAdBlockStart => "Distributor Ad Block Start",
            DistributorAdBlockEnd => "Distributor Ad Block End",
            NetworkStart => "Network Start",
            NetworkEnd => "Network End",
        }
    }
}
