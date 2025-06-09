//! SCTE-35 descriptor types and parsing.
//!
//! This module contains structures and functions for handling SCTE-35 descriptors,
//! which provide additional metadata about splice operations.

use crate::types::SegmentationType;
use crate::upid::{format_base64, format_isan, format_uuid, SegmentationUpidType};
use std::time::Duration;

/// Represents different types of splice descriptors with parsed content.
///
/// This enum provides structured access to descriptor data, with full parsing
/// for supported descriptor types and raw bytes for unsupported types.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "descriptor_type"))]
pub enum SpliceDescriptor {
    /// Segmentation descriptor (tag 0x02) - fully parsed
    Segmentation(SegmentationDescriptor),
    /// Avail descriptor (tag 0x00) - for ad availability
    Avail(AvailDescriptor),
    /// DTMF descriptor (tag 0x01) - for DTMF signaling
    Dtmf(DtmfDescriptor),
    /// Time descriptor (tag 0x03) - for time synchronization
    Time(TimeDescriptor),
    /// Audio descriptor (tag 0x04) - for audio component information
    Audio(AudioDescriptor),
    /// Unknown or unsupported descriptor type with raw bytes
    Unknown {
        /// Descriptor tag
        tag: u8,
        /// Length of descriptor data
        length: u8,
        /// Raw descriptor bytes
        #[cfg_attr(
            feature = "serde",
            serde(
                serialize_with = "crate::serde::serialize_bytes",
                deserialize_with = "crate::serde::deserialize_bytes"
            )
        )]
        data: Vec<u8>,
    },
}

impl SpliceDescriptor {
    /// Returns the descriptor tag.
    pub fn tag(&self) -> u8 {
        match self {
            SpliceDescriptor::Segmentation(_) => 0x02,
            SpliceDescriptor::Avail(_) => 0x00,
            SpliceDescriptor::Dtmf(_) => 0x01,
            SpliceDescriptor::Time(_) => 0x03,
            SpliceDescriptor::Audio(_) => 0x04,
            SpliceDescriptor::Unknown { tag, .. } => *tag,
        }
    }

    /// Returns the descriptor length.
    pub fn length(&self) -> u8 {
        match self {
            SpliceDescriptor::Segmentation(_) => {
                // For segmentation descriptors, we calculate based on the actual content
                // This is a simplified calculation - real implementation would serialize back
                33 // Minimum segmentation descriptor length
            }
            SpliceDescriptor::Avail(desc) => 4 + desc.provider_avail_id.len() as u8,
            SpliceDescriptor::Dtmf(desc) => 4 + desc.dtmf_chars.len() as u8,
            SpliceDescriptor::Time(_) => 4 + 6 + 4 + 2, // identifier + tai_seconds + tai_ns + utc_offset
            SpliceDescriptor::Audio(desc) => 4 + desc.audio_components.len() as u8,
            SpliceDescriptor::Unknown { length, .. } => *length,
        }
    }

    /// Returns raw descriptor bytes if available (for unknown descriptor types).
    pub fn raw_bytes(&self) -> Option<&[u8]> {
        match self {
            SpliceDescriptor::Segmentation(_) => None,
            SpliceDescriptor::Avail(_) => None,
            SpliceDescriptor::Dtmf(_) => None,
            SpliceDescriptor::Time(_) => None,
            SpliceDescriptor::Audio(_) => None,
            SpliceDescriptor::Unknown { data, .. } => Some(data),
        }
    }

    /// Attempts to interpret descriptor bytes as a UTF-8 string.
    ///
    /// This is useful for descriptors that contain text-based data.
    /// For segmentation descriptors, this will attempt to interpret the UPID as a string.
    ///
    /// # Returns
    ///
    /// - `Some(String)` if the descriptor can be converted to a readable string
    /// - `None` if the descriptor doesn't support string conversion
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35::SpliceDescriptor;
    ///
    /// // For an unknown descriptor with raw bytes
    /// let descriptor = SpliceDescriptor::Unknown {
    ///     tag: 0x00,
    ///     length: 5,
    ///     data: vec![0x48, 0x65, 0x6c, 0x6c, 0x6f], // "Hello"
    /// };
    ///
    /// assert_eq!(descriptor.as_str(), Some("Hello".to_string()));
    /// ```
    pub fn as_str(&self) -> Option<String> {
        match self {
            SpliceDescriptor::Segmentation(seg_desc) => seg_desc.upid_as_string(),
            SpliceDescriptor::Avail(avail_desc) => {
                std::str::from_utf8(&avail_desc.provider_avail_id)
                    .ok()
                    .map(|s| s.to_string())
            }
            SpliceDescriptor::Dtmf(dtmf_desc) => std::str::from_utf8(&dtmf_desc.dtmf_chars)
                .ok()
                .map(|s| s.to_string()),
            SpliceDescriptor::Time(_) => None, // Time data not interpretable as string
            SpliceDescriptor::Audio(_) => None, // Audio data not interpretable as string
            SpliceDescriptor::Unknown { data, .. } => {
                std::str::from_utf8(data).ok().map(|s| s.to_string())
            }
        }
    }
}

/// Represents a parsed segmentation descriptor (tag 0x02).
///
/// Segmentation descriptors provide detailed information about content segments,
/// including timing, UPID data, and segmentation types. This struct provides
/// structured access to the segmentation descriptor fields.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct SegmentationDescriptor {
    /// Segmentation event identifier
    pub segmentation_event_id: u32,
    /// Indicates if this event should be cancelled
    pub segmentation_event_cancel_indicator: bool,
    /// Program segmentation flag
    pub program_segmentation_flag: bool,
    /// Segmentation duration flag
    pub segmentation_duration_flag: bool,
    /// Delivery not restricted flag
    pub delivery_not_restricted_flag: bool,
    /// Web delivery allowed flag (present when delivery_not_restricted_flag is false)
    pub web_delivery_allowed_flag: Option<bool>,
    /// No regional blackout flag (present when delivery_not_restricted_flag is false)
    pub no_regional_blackout_flag: Option<bool>,
    /// Archive allowed flag (present when delivery_not_restricted_flag is false)
    pub archive_allowed_flag: Option<bool>,
    /// Device restrictions (present when delivery_not_restricted_flag is false)
    pub device_restrictions: Option<u8>,
    /// Segmentation duration in 90kHz ticks (present when segmentation_duration_flag is true)
    pub segmentation_duration: Option<u64>,
    /// UPID type identifier
    pub segmentation_upid_type: SegmentationUpidType,
    /// Length of UPID data in bytes
    pub segmentation_upid_length: u8,
    /// Raw UPID data bytes
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "crate::serde::serialize_bytes",
            deserialize_with = "crate::serde::deserialize_bytes"
        )
    )]
    pub segmentation_upid: Vec<u8>,
    /// Segmentation type identifier
    pub segmentation_type_id: u8,
    /// Human-readable segmentation type (derived from segmentation_type_id)
    pub segmentation_type: SegmentationType,
    /// Segment number
    pub segment_num: u8,
    /// Expected number of segments
    pub segments_expected: u8,
    /// Sub-segment number (present for certain segmentation types)
    pub sub_segment_num: Option<u8>,
    /// Expected number of sub-segments (present for certain segmentation types)
    pub sub_segments_expected: Option<u8>,
}

impl SegmentationDescriptor {
    /// Returns the UPID as a human-readable string if possible.
    ///
    /// This method attempts to convert the raw UPID bytes into a meaningful
    /// string representation based on the UPID type.
    ///
    /// # Returns
    ///
    /// - `Some(String)` if the UPID can be converted to a readable string
    /// - `None` if the UPID type doesn't support string conversion or the data is malformed
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35::{SegmentationDescriptor, SegmentationUpidType, SegmentationType};
    ///
    /// let descriptor = SegmentationDescriptor {
    ///     segmentation_event_id: 1,
    ///     segmentation_event_cancel_indicator: false,
    ///     program_segmentation_flag: true,
    ///     segmentation_duration_flag: false,
    ///     delivery_not_restricted_flag: true,
    ///     web_delivery_allowed_flag: None,
    ///     no_regional_blackout_flag: None,
    ///     archive_allowed_flag: None,
    ///     device_restrictions: None,
    ///     segmentation_duration: None,
    ///     segmentation_upid_type: SegmentationUpidType::AdID,
    ///     segmentation_upid_length: 12,
    ///     segmentation_upid: b"ABCD01234567".to_vec(),
    ///     segmentation_type_id: 0x30,
    ///     segmentation_type: SegmentationType::from_id(0x30),
    ///     segment_num: 1,
    ///     segments_expected: 1,
    ///     sub_segment_num: None,
    ///     sub_segments_expected: None,
    /// };
    ///
    /// assert_eq!(descriptor.upid_as_string(), Some("ABCD01234567".to_string()));
    /// ```
    pub fn upid_as_string(&self) -> Option<String> {
        match self.segmentation_upid_type {
            SegmentationUpidType::URI
            | SegmentationUpidType::MPU
            | SegmentationUpidType::AdID
            | SegmentationUpidType::TID => std::str::from_utf8(&self.segmentation_upid)
                .ok()
                .map(|s| s.to_string()),
            SegmentationUpidType::UUID => {
                if self.segmentation_upid.len() == 16 {
                    Some(format_uuid(&self.segmentation_upid))
                } else {
                    None
                }
            }
            SegmentationUpidType::ISAN => {
                if self.segmentation_upid.len() >= 12 {
                    Some(format_isan(&self.segmentation_upid))
                } else {
                    None
                }
            }
            // For other types, return base64 representation for now
            _ => {
                if !self.segmentation_upid.is_empty() {
                    Some(format_base64(&self.segmentation_upid))
                } else {
                    None
                }
            }
        }
    }

    /// Returns a description of the UPID type.
    ///
    /// This is a convenience method that returns the string representation of the UPID type.
    pub fn upid_type_description(&self) -> String {
        self.segmentation_upid_type.to_string()
    }

    /// Converts the segmentation duration to a [`std::time::Duration`] if present.
    ///
    /// Segmentation durations are stored as 90kHz ticks in SCTE-35 messages.
    /// This method converts those ticks to a standard Rust Duration.
    ///
    /// # Returns
    ///
    /// - `Some(Duration)` if a segmentation duration is specified
    /// - `None` if no duration is specified (segmentation_duration_flag is false)
    pub fn duration(&self) -> Option<Duration> {
        self.segmentation_duration.map(|ticks| {
            let seconds = ticks / 90_000;
            let nanos = ((ticks % 90_000) * 1_000_000_000) / 90_000;
            Duration::new(seconds, nanos as u32)
        })
    }

    /// Returns a human-readable description of the segmentation type.
    ///
    /// This is a convenience method that returns the string representation of the segmentation type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35::{SegmentationDescriptor, SegmentationUpidType, SegmentationType};
    ///
    /// let descriptor = SegmentationDescriptor {
    ///     segmentation_event_id: 1,
    ///     segmentation_event_cancel_indicator: false,
    ///     program_segmentation_flag: true,
    ///     segmentation_duration_flag: false,
    ///     delivery_not_restricted_flag: true,
    ///     web_delivery_allowed_flag: None,
    ///     no_regional_blackout_flag: None,
    ///     archive_allowed_flag: None,
    ///     device_restrictions: None,
    ///     segmentation_duration: None,
    ///     segmentation_upid_type: SegmentationUpidType::NotUsed,
    ///     segmentation_upid_length: 0,
    ///     segmentation_upid: vec![],
    ///     segmentation_type_id: 0x30,
    ///     segmentation_type: SegmentationType::ProviderAdvertisementStart,
    ///     segment_num: 1,
    ///     segments_expected: 1,
    ///     sub_segment_num: None,
    ///     sub_segments_expected: None,
    /// };
    ///
    /// assert_eq!(descriptor.segmentation_type_description(), "Provider Advertisement Start");
    /// ```
    pub fn segmentation_type_description(&self) -> String {
        self.segmentation_type.to_string()
    }

    /// Creates a new SegmentationDescriptor with the segmentation_type field automatically
    /// populated from the segmentation_type_id.
    ///
    /// This is a convenience constructor that ensures the human-readable segmentation type
    /// is always consistent with the numeric ID.
    ///
    /// # Arguments
    ///
    /// All the same fields as the struct, except `segmentation_type` which is derived
    /// from `segmentation_type_id`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35::{SegmentationDescriptor, SegmentationUpidType, SegmentationType};
    ///
    /// let descriptor = SegmentationDescriptor::new(
    ///     1,                                    // segmentation_event_id
    ///     false,                               // segmentation_event_cancel_indicator
    ///     true,                                // program_segmentation_flag
    ///     false,                               // segmentation_duration_flag
    ///     true,                                // delivery_not_restricted_flag
    ///     None,                                // web_delivery_allowed_flag
    ///     None,                                // no_regional_blackout_flag
    ///     None,                                // archive_allowed_flag
    ///     None,                                // device_restrictions
    ///     None,                                // segmentation_duration
    ///     SegmentationUpidType::NotUsed,       // segmentation_upid_type
    ///     0,                                   // segmentation_upid_length
    ///     vec![],                              // segmentation_upid
    ///     0x30,                                // segmentation_type_id
    ///     1,                                   // segment_num
    ///     1,                                   // segments_expected
    ///     None,                                // sub_segment_num
    ///     None,                                // sub_segments_expected
    /// );
    ///
    /// // The segmentation_type is automatically set to ProviderAdvertisementStart
    /// assert_eq!(descriptor.segmentation_type, SegmentationType::ProviderAdvertisementStart);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        segmentation_event_id: u32,
        segmentation_event_cancel_indicator: bool,
        program_segmentation_flag: bool,
        segmentation_duration_flag: bool,
        delivery_not_restricted_flag: bool,
        web_delivery_allowed_flag: Option<bool>,
        no_regional_blackout_flag: Option<bool>,
        archive_allowed_flag: Option<bool>,
        device_restrictions: Option<u8>,
        segmentation_duration: Option<u64>,
        segmentation_upid_type: SegmentationUpidType,
        segmentation_upid_length: u8,
        segmentation_upid: Vec<u8>,
        segmentation_type_id: u8,
        segment_num: u8,
        segments_expected: u8,
        sub_segment_num: Option<u8>,
        sub_segments_expected: Option<u8>,
    ) -> Self {
        Self {
            segmentation_event_id,
            segmentation_event_cancel_indicator,
            program_segmentation_flag,
            segmentation_duration_flag,
            delivery_not_restricted_flag,
            web_delivery_allowed_flag,
            no_regional_blackout_flag,
            archive_allowed_flag,
            device_restrictions,
            segmentation_duration,
            segmentation_upid_type,
            segmentation_upid_length,
            segmentation_upid,
            segmentation_type_id,
            segmentation_type: SegmentationType::from_id(segmentation_type_id),
            segment_num,
            segments_expected,
            sub_segment_num,
            sub_segments_expected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splice_descriptor_tag() {
        let seg_desc = SegmentationDescriptor {
            segmentation_event_id: 1,
            segmentation_event_cancel_indicator: false,
            program_segmentation_flag: true,
            segmentation_duration_flag: false,
            delivery_not_restricted_flag: true,
            web_delivery_allowed_flag: None,
            no_regional_blackout_flag: None,
            archive_allowed_flag: None,
            device_restrictions: None,
            segmentation_duration: None,
            segmentation_upid_type: SegmentationUpidType::NotUsed,
            segmentation_upid_length: 0,
            segmentation_upid: vec![],
            segmentation_type_id: 0x30,
            segmentation_type: SegmentationType::ProviderAdvertisementStart,
            segment_num: 1,
            segments_expected: 1,
            sub_segment_num: None,
            sub_segments_expected: None,
        };

        let descriptor = SpliceDescriptor::Segmentation(seg_desc);
        assert_eq!(descriptor.tag(), 0x02);

        let unknown = SpliceDescriptor::Unknown {
            tag: 0xFF,
            length: 0,
            data: vec![],
        };
        assert_eq!(unknown.tag(), 0xFF);
    }

    #[test]
    fn test_splice_descriptor_as_str() {
        let unknown = SpliceDescriptor::Unknown {
            tag: 0x00,
            length: 5,
            data: vec![0x48, 0x65, 0x6c, 0x6c, 0x6f], // "Hello"
        };
        assert_eq!(unknown.as_str(), Some("Hello".to_string()));

        let unknown_binary = SpliceDescriptor::Unknown {
            tag: 0x00,
            length: 3,
            data: vec![0xFF, 0xFE, 0xFD], // Not valid UTF-8
        };
        assert_eq!(unknown_binary.as_str(), None);
    }

    #[test]
    fn test_segmentation_descriptor_duration() {
        let desc = SegmentationDescriptor {
            segmentation_event_id: 1,
            segmentation_event_cancel_indicator: false,
            program_segmentation_flag: true,
            segmentation_duration_flag: true,
            delivery_not_restricted_flag: true,
            web_delivery_allowed_flag: None,
            no_regional_blackout_flag: None,
            archive_allowed_flag: None,
            device_restrictions: None,
            segmentation_duration: Some(900_000), // 10 seconds
            segmentation_upid_type: SegmentationUpidType::NotUsed,
            segmentation_upid_length: 0,
            segmentation_upid: vec![],
            segmentation_type_id: 0x30,
            segmentation_type: SegmentationType::ProviderAdvertisementStart,
            segment_num: 1,
            segments_expected: 1,
            sub_segment_num: None,
            sub_segments_expected: None,
        };

        assert_eq!(desc.duration(), Some(Duration::from_secs(10)));

        let desc_no_duration = SegmentationDescriptor {
            segmentation_duration: None,
            ..desc
        };
        assert_eq!(desc_no_duration.duration(), None);
    }
}

/// Avail descriptor for ad availability information.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AvailDescriptor {
    /// Descriptor identifier (typically 0x43554549 "CUEI")
    pub identifier: u32,
    /// Provider-specific avail identifier
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "crate::serde::serialize_bytes",
            deserialize_with = "crate::serde::deserialize_bytes"
        )
    )]
    pub provider_avail_id: Vec<u8>,
}

/// DTMF descriptor for DTMF tone signaling.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DtmfDescriptor {
    /// Descriptor identifier (typically 0x43554549 "CUEI")
    pub identifier: u32,
    /// Preroll duration in 90kHz ticks
    pub preroll: u8,
    /// DTMF character count
    pub dtmf_count: u8,
    /// DTMF characters
    pub dtmf_chars: Vec<u8>,
}

/// Time descriptor for time synchronization.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimeDescriptor {
    /// Descriptor identifier (typically 0x43554549 "CUEI")
    pub identifier: u32,
    /// TAI seconds (6 bytes)
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "crate::serde::serialize_bytes",
            deserialize_with = "crate::serde::deserialize_bytes"
        )
    )]
    pub tai_seconds: Vec<u8>,
    /// TAI nanoseconds (4 bytes)
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "crate::serde::serialize_bytes",
            deserialize_with = "crate::serde::deserialize_bytes"
        )
    )]
    pub tai_ns: Vec<u8>,
    /// UTC offset (2 bytes)
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "crate::serde::serialize_bytes",
            deserialize_with = "crate::serde::deserialize_bytes"
        )
    )]
    pub utc_offset: Vec<u8>,
}

/// Audio descriptor for audio component information.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AudioDescriptor {
    /// Descriptor identifier (typically 0x43554549 "CUEI")
    pub identifier: u32,
    /// Audio component data
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "crate::serde::serialize_bytes",
            deserialize_with = "crate::serde::deserialize_bytes"
        )
    )]
    pub audio_components: Vec<u8>,
}
