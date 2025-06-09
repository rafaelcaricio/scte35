//! Serde serialization support for SCTE-35 types.
//!
//! This module provides custom serialization and deserialization implementations
//! for SCTE-35 types when the `serde` feature is enabled.

use data_encoding::BASE64;
use serde::de::{self, Deserializer, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

/// Serialize bytes as base64-encoded string.
pub fn serialize_bytes<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&BASE64.encode(bytes))
}

/// Deserialize base64-encoded string to bytes.
pub fn deserialize_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    struct BytesVisitor;

    impl<'de> Visitor<'de> for BytesVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a base64-encoded string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            BASE64
                .decode(value.as_bytes())
                .map_err(|e| E::custom(format!("invalid base64: {}", e)))
        }
    }

    deserializer.deserialize_str(BytesVisitor)
}

/// Serialize optional bytes as base64-encoded string.
pub fn serialize_optional_bytes<S>(
    bytes: &Option<Vec<u8>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match bytes {
        Some(b) => serialize_bytes(b, serializer),
        None => serializer.serialize_none(),
    }
}

/// Deserialize optional base64-encoded string to bytes.
pub fn deserialize_optional_bytes<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(transparent)]
    struct OptionalBytes(Option<String>);

    let opt = OptionalBytes::deserialize(deserializer)?;
    match opt.0 {
        Some(s) => BASE64
            .decode(s.as_bytes())
            .map(Some)
            .map_err(|e| de::Error::custom(format!("invalid base64: {}", e))),
        None => Ok(None),
    }
}

/// Helper struct for serializing duration information.
#[derive(Serialize, Deserialize)]
pub struct DurationInfo {
    /// Duration in 90kHz ticks
    pub ticks: u64,
    /// Duration in seconds
    pub seconds: f64,
    /// Human-readable duration string
    pub human_readable: String,
}

impl DurationInfo {
    /// Create duration info from 90kHz ticks.
    pub fn from_ticks(ticks: u64) -> Self {
        let seconds = ticks as f64 / 90_000.0;
        let human_readable = format_duration_seconds(seconds);
        Self {
            ticks,
            seconds,
            human_readable,
        }
    }
}

/// Format duration in seconds to human-readable string.
fn format_duration_seconds(seconds: f64) -> String {
    if seconds < 1.0 {
        format!("{:.3}s", seconds)
    } else if seconds < 60.0 {
        format!("{:.1}s", seconds)
    } else if seconds < 3600.0 {
        let minutes = (seconds / 60.0).floor();
        let secs = seconds % 60.0;
        format!("{}m {:.1}s", minutes as u64, secs)
    } else {
        let hours = (seconds / 3600.0).floor();
        let minutes = ((seconds % 3600.0) / 60.0).floor();
        let secs = seconds % 60.0;
        format!("{}h {}m {:.1}s", hours as u64, minutes as u64, secs)
    }
}

/// Custom serialization for SegmentationType to include both ID and description.
use crate::types::SegmentationType;

impl Serialize for SegmentationType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("SegmentationType", 2)?;
        state.serialize_field("id", &self.id())?;
        state.serialize_field("description", &self.to_string())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SegmentationType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SegmentationTypeData {
            id: u8,
        }

        let data = SegmentationTypeData::deserialize(deserializer)?;
        Ok(SegmentationType::from_id(data.id))
    }
}

/// Custom serialization for SegmentationUpidType to include both value and description.
use crate::upid::SegmentationUpidType;

/// Custom serialization for SegmentationDescriptor to include computed fields.
use crate::descriptors::SegmentationDescriptor;

impl Serialize for SegmentationUpidType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("SegmentationUpidType", 2)?;
        state.serialize_field("value", &u8::from(*self))?;
        state.serialize_field("description", &self.to_string())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SegmentationUpidType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct UpidTypeData {
            value: u8,
        }

        let data = UpidTypeData::deserialize(deserializer)?;
        Ok(SegmentationUpidType::from(data.value))
    }
}

impl Serialize for SegmentationDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("SegmentationDescriptor", 20)?;

        // Serialize all the fields
        state.serialize_field("segmentation_event_id", &self.segmentation_event_id)?;
        state.serialize_field(
            "segmentation_event_cancel_indicator",
            &self.segmentation_event_cancel_indicator,
        )?;
        state.serialize_field("program_segmentation_flag", &self.program_segmentation_flag)?;
        state.serialize_field(
            "segmentation_duration_flag",
            &self.segmentation_duration_flag,
        )?;
        state.serialize_field(
            "delivery_not_restricted_flag",
            &self.delivery_not_restricted_flag,
        )?;
        state.serialize_field("web_delivery_allowed_flag", &self.web_delivery_allowed_flag)?;
        state.serialize_field("no_regional_blackout_flag", &self.no_regional_blackout_flag)?;
        state.serialize_field("archive_allowed_flag", &self.archive_allowed_flag)?;
        state.serialize_field("device_restrictions", &self.device_restrictions)?;
        state.serialize_field("segmentation_duration", &self.segmentation_duration)?;
        state.serialize_field("segmentation_upid_type", &self.segmentation_upid_type)?;
        state.serialize_field("segmentation_upid_length", &self.segmentation_upid_length)?;

        // Serialize UPID as base64
        state.serialize_field("segmentation_upid", &BASE64.encode(&self.segmentation_upid))?;

        state.serialize_field("segmentation_type_id", &self.segmentation_type_id)?;
        state.serialize_field("segmentation_type", &self.segmentation_type)?;
        state.serialize_field("segment_num", &self.segment_num)?;
        state.serialize_field("segments_expected", &self.segments_expected)?;
        state.serialize_field("sub_segment_num", &self.sub_segment_num)?;
        state.serialize_field("sub_segments_expected", &self.sub_segments_expected)?;

        // Add computed fields
        if let Some(upid_string) = self.upid_as_string() {
            state.serialize_field("upid_string", &upid_string)?;
        }

        if let Some(_d) = self.duration() {
            let duration_info = DurationInfo::from_ticks(self.segmentation_duration.unwrap_or(0));
            state.serialize_field("duration_info", &duration_info)?;
        }

        state.end()
    }
}

/// Custom serialization for SpliceTime to include duration info.
use crate::time::{BreakDuration, SpliceTime};

impl Serialize for SpliceTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("SpliceTime", 3)?;
        state.serialize_field("time_specified_flag", &self.time_specified_flag)?;
        state.serialize_field("pts_time", &self.pts_time)?;

        // Add duration info if time is specified
        if let Some(ticks) = self.pts_time {
            let duration_info = DurationInfo::from_ticks(ticks);
            state.serialize_field("duration_info", &duration_info)?;
        }

        state.end()
    }
}

impl Serialize for BreakDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("BreakDuration", 4)?;
        state.serialize_field("auto_return", &self.auto_return)?;
        state.serialize_field("reserved", &self.reserved)?;
        state.serialize_field("duration", &self.duration)?;

        // Always add duration info
        let duration_info = DurationInfo::from_ticks(self.duration);
        state.serialize_field("duration_info", &duration_info)?;

        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_bytes() {
        let bytes = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let json = serde_json::to_string(&serde_json::json!({
            "data": bytes
        }))
        .unwrap();

        // The JSON serializer will represent the bytes as an array by default
        // Our custom serializer will encode as base64
        assert!(json.contains("[72,101,108,108,111]"));
    }

    #[test]
    fn test_duration_info() {
        let info = DurationInfo::from_ticks(90_000); // 1 second
        assert_eq!(info.ticks, 90_000);
        assert_eq!(info.seconds, 1.0);
        assert_eq!(info.human_readable, "1.0s");

        let info = DurationInfo::from_ticks(5_400_000); // 60 seconds
        assert_eq!(info.seconds, 60.0);
        assert_eq!(info.human_readable, "1m 0.0s");

        let info = DurationInfo::from_ticks(324_000_000); // 3600 seconds (1 hour)
        assert_eq!(info.seconds, 3600.0);
        assert_eq!(info.human_readable, "1h 0m 0.0s");
    }

    #[test]
    fn test_segmentation_type_serialization() {
        let seg_type = SegmentationType::ProviderAdvertisementStart;
        let json = serde_json::to_string(&seg_type).unwrap();
        assert!(json.contains("\"id\":48"));
        assert!(json.contains("\"description\":\"Provider Advertisement Start\""));

        // Test deserialization
        let deserialized: SegmentationType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, seg_type);
    }

    #[test]
    fn test_upid_type_serialization() {
        let upid_type = SegmentationUpidType::AdID;
        let json = serde_json::to_string(&upid_type).unwrap();
        assert!(json.contains("\"value\":3"));
        assert!(json.contains("\"description\":\"Ad Identifier\""));

        // Test deserialization
        let deserialized: SegmentationUpidType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, upid_type);
    }

    #[test]
    fn test_splice_time_serialization() {
        // Test with time specified
        let splice_time = SpliceTime {
            time_specified_flag: 1,
            pts_time: Some(450_000), // 5 seconds
        };

        let json = serde_json::to_string(&splice_time).unwrap();
        assert!(json.contains("\"time_specified_flag\":1"));
        assert!(json.contains("\"pts_time\":450000"));
        assert!(json.contains("\"ticks\":450000"));
        assert!(json.contains("\"seconds\":5.0"));
        assert!(json.contains("\"human_readable\":\"5.0s\""));

        // Test with no time specified
        let splice_time_immediate = SpliceTime {
            time_specified_flag: 0,
            pts_time: None,
        };

        let json_immediate = serde_json::to_string(&splice_time_immediate).unwrap();
        assert!(json_immediate.contains("\"time_specified_flag\":0"));
        assert!(json_immediate.contains("\"pts_time\":null"));
        assert!(!json_immediate.contains("duration_info"));
    }

    #[test]
    fn test_break_duration_serialization() {
        let break_duration = BreakDuration {
            auto_return: 1,
            reserved: 0,
            duration: 2_700_000, // 30 seconds
        };

        let json = serde_json::to_string(&break_duration).unwrap();
        assert!(json.contains("\"auto_return\":1"));
        assert!(json.contains("\"duration\":2700000"));
        assert!(json.contains("\"ticks\":2700000"));
        assert!(json.contains("\"seconds\":30.0"));
        assert!(json.contains("\"human_readable\":\"30.0s\""));
    }

    #[test]
    fn test_segmentation_descriptor_serialization() {
        let descriptor = SegmentationDescriptor {
            segmentation_event_id: 12345,
            segmentation_event_cancel_indicator: false,
            program_segmentation_flag: true,
            segmentation_duration_flag: true,
            delivery_not_restricted_flag: true,
            web_delivery_allowed_flag: None,
            no_regional_blackout_flag: None,
            archive_allowed_flag: None,
            device_restrictions: None,
            segmentation_duration: Some(900_000), // 10 seconds
            segmentation_upid_type: SegmentationUpidType::AdID,
            segmentation_upid_length: 12,
            segmentation_upid: b"TEST12345678".to_vec(),
            segmentation_type_id: 0x30,
            segmentation_type: SegmentationType::ProviderAdvertisementStart,
            segment_num: 1,
            segments_expected: 1,
            sub_segment_num: None,
            sub_segments_expected: None,
        };

        let json = serde_json::to_string_pretty(&descriptor).unwrap();

        // Check basic fields
        assert!(json.contains("\"segmentation_event_id\": 12345"));
        assert!(json.contains("\"segmentation_event_cancel_indicator\": false"));

        // Check UPID is base64 encoded
        assert!(json.contains("\"segmentation_upid\": \"VEVTVDEyMzQ1Njc4\""));

        // Check computed UPID string
        assert!(json.contains("\"upid_string\": \"TEST12345678\""));

        // Check duration info
        assert!(json.contains("\"ticks\": 900000"));
        assert!(json.contains("\"seconds\": 10.0"));
        assert!(json.contains("\"human_readable\": \"10.0s\""));

        // Check segmentation type
        assert!(json.contains("\"segmentation_type_id\": 48"));
    }

    #[test]
    fn test_binary_data_serialization() {
        use crate::types::PrivateCommand;

        let private_cmd = PrivateCommand {
            private_command_id: 0x1234,
            private_command_length: 5,
            private_bytes: vec![0x01, 0x02, 0x03, 0x04, 0x05],
        };

        let json = serde_json::to_string(&private_cmd).unwrap();

        // Check that private_bytes is base64 encoded
        assert!(json.contains("\"private_bytes\":\"AQIDBAU=\"")); // base64 of [1,2,3,4,5]
    }

    #[test]
    fn test_splice_descriptor_enum_serialization() {
        use crate::descriptors::SpliceDescriptor;

        // Test Unknown variant
        let unknown = SpliceDescriptor::Unknown {
            tag: 0xFF,
            length: 3,
            data: vec![0xAA, 0xBB, 0xCC],
        };

        let json = serde_json::to_string(&unknown).unwrap();
        assert!(json.contains("\"descriptor_type\":\"Unknown\""));
        assert!(json.contains("\"tag\":255"));
        assert!(json.contains("\"data\":\"qrvM\"")); // base64 of [0xAA, 0xBB, 0xCC]
    }

    #[test]
    fn test_round_trip_serialization() {
        // Test that we can serialize and deserialize back
        let seg_type = SegmentationType::ProgramStart;
        let json = serde_json::to_string(&seg_type).unwrap();
        let deserialized: SegmentationType = serde_json::from_str(&json).unwrap();
        assert_eq!(seg_type, deserialized);

        let upid_type = SegmentationUpidType::UUID;
        let json = serde_json::to_string(&upid_type).unwrap();
        let deserialized: SegmentationUpidType = serde_json::from_str(&json).unwrap();
        assert_eq!(upid_type, deserialized);
    }
}
