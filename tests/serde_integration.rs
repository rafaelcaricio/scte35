//! Integration tests for serde serialization/deserialization

#[cfg(feature = "serde")]
#[cfg(test)]
mod tests {
    use data_encoding::BASE64;
    use scte35::*;

    #[test]
    fn test_complete_message_serialization() {
        // Real SCTE-35 message from the existing tests
        let base64_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
        let buffer = BASE64.decode(base64_message.as_bytes()).unwrap();

        let section = parse_splice_info_section(&buffer).unwrap();

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&section).unwrap();

        // Print for debugging
        println!("JSON: {json}");

        // Basic structure checks
        assert!(json.contains("\"table_id\": 252"));
        assert!(json.contains("\"type\": \"TimeSignal\""));

        // The TimeSignal contains a splice_time with pts_time
        assert!(json.contains("\"splice_time\""));
        assert!(json.contains("\"pts_time\""));
        assert!(json.contains("\"time_specified_flag\""));

        // Check duration info is included
        assert!(json.contains("\"duration_info\""));
        assert!(json.contains("\"ticks\""));
        assert!(json.contains("\"human_readable\""));

        // Deserialize back
        let deserialized: SpliceInfoSection = serde_json::from_str(&json).unwrap();

        // Compare key fields (we can't compare the entire struct due to custom serialization)
        assert_eq!(section.table_id, deserialized.table_id);
        assert_eq!(
            section.splice_command_type,
            deserialized.splice_command_type
        );
        assert_eq!(section.crc_32, deserialized.crc_32);
    }

    #[test]
    fn test_segmentation_descriptor_json() {
        // Use the placement opportunity end example from our tests
        let base64_message = "/DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=";
        let buffer = BASE64.decode(base64_message.as_bytes()).unwrap();

        let section = parse_splice_info_section(&buffer).unwrap();

        let json = serde_json::to_string_pretty(&section).unwrap();

        // Print JSON for debugging
        println!("Segmentation JSON: {json}");

        // Check segmentation descriptor fields
        assert!(json.contains("\"splice_descriptors\""));
        assert!(json.contains("\"descriptor_type\": \"Segmentation\""));
        assert!(json.contains("\"segmentation_event_id\""));
        assert!(json.contains("\"segmentation_type\""));

        // Check specific values
        assert!(json.contains("\"segmentation_type_id\": 53")); // 0x35 = 53
        assert!(json.contains("\"description\": \"Provider Placement Opportunity End\""));

        // Check UPID is base64 encoded
        assert!(json.contains("\"segmentation_upid\":"));
    }

    #[test]
    fn test_binary_fields_base64() {
        use scte35::types::PrivateCommand;

        let private_cmd = PrivateCommand {
            private_command_id: 0xABCD,
            private_command_length: 4,
            private_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };

        let json = serde_json::to_string(&private_cmd).unwrap();

        // Check that private_bytes is base64 encoded
        assert!(json.contains("\"private_bytes\":\"3q2+7w==\"")); // base64 of [0xDE, 0xAD, 0xBE, 0xEF]

        // Deserialize back
        let deserialized: PrivateCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(private_cmd.private_bytes, deserialized.private_bytes);
    }

    #[test]
    fn test_enum_serialization() {
        let seg_type = SegmentationType::ProviderAdvertisementStart;
        let json = serde_json::to_string(&seg_type).unwrap();

        // Should include both ID and description
        assert_eq!(
            json,
            "{\"id\":48,\"description\":\"Provider Advertisement Start\"}"
        );

        let upid_type = SegmentationUpidType::AdID;
        let json = serde_json::to_string(&upid_type).unwrap();

        // Should include both value and description
        assert_eq!(json, "{\"value\":3,\"description\":\"Ad Identifier\"}");
    }

    #[test]
    fn test_mpu_upid_serialization() {
        // Test message with MPU UPID type
        let base64_message = "/DAsAAAAAAAAAP/wBQb+7YaD1QAWAhRDVUVJAADc8X+/DAVPVkxZSSIAAJ6Gk2Q=";
        let buffer = BASE64.decode(base64_message.as_bytes()).unwrap();

        let section = parse_splice_info_section(&buffer).unwrap();
        let json = serde_json::to_string_pretty(&section).unwrap();

        // Print JSON for debugging
        println!("MPU UPID JSON: {json}");

        // Check that MPU UPID is properly serialized
        assert!(json.contains("\"segmentation_upid_type\": {"));
        assert!(json.contains("\"value\": 12")); // MPU is 0x0C = 12
        assert!(json.contains("\"description\": \"MPU (Media Processing Unit)\""));

        // Check UPID data is base64 encoded
        assert!(json.contains("\"segmentation_upid\": \"T1ZMWUk=\"")); // base64 of "OVLYI"

        // Check computed UPID string
        assert!(json.contains("\"upid_string\": \"OVLYI\""));

        // Check segmentation type
        assert!(json.contains("\"segmentation_type_id\": 34")); // 0x22 = 34
        assert!(json.contains("\"description\": \"Break Start\""));

        // Verify round-trip
        let deserialized: scte35::SpliceInfoSection = serde_json::from_str(&json).unwrap();
        assert_eq!(section.crc_32, deserialized.crc_32);
    }
}
