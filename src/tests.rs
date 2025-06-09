use super::*;
use data_encoding::BASE64;
use std::time::Duration;

#[test]
fn test_time_signal_command() {
    // Time Signal example from threefive: '/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A=='
    let time_signal_base64 = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
    let buffer = BASE64
        .decode(time_signal_base64.as_bytes())
        .expect("Failed to decode base64 string");

    let section =
        parse_splice_info_section(&buffer).expect("Failed to parse time_signal SpliceInfoSection");

    // Validate header
    assert_eq!(section.table_id, 0xFC, "Table ID should be 0xFC");
    assert_eq!(
        section.splice_command_type, 0x06,
        "Command type should be 0x06 (time_signal)"
    );

    // Validate command
    match section.splice_command {
        SpliceCommand::TimeSignal(ref cmd) => {
            assert_eq!(
                cmd.splice_time.time_specified_flag, 1,
                "Time should be specified"
            );
            assert!(
                cmd.splice_time.pts_time.is_some(),
                "PTS time should be present"
            );

            // Verify time conversion
            if let Some(duration) = cmd.splice_time.to_duration() {
                // PTS time is 1111111101, which is about 12345 seconds
                assert!(duration.as_secs() > 12000 && duration.as_secs() < 13000);
            }
        }
        _ => panic!("Expected TimeSignal command"),
    }
}

#[test]
fn test_time_signal_with_descriptors() {
    // Time Signal with descriptors: '/DAgAAAAAAAAAP/wBQb+Qjo1vQAKAAhDVUVJAAAE0iVuWvA='
    let time_signal_desc_base64 = "/DAgAAAAAAAAAP/wBQb+Qjo1vQAKAAhDVUVJAAAE0iVuWvA=";
    let buffer = BASE64
        .decode(time_signal_desc_base64.as_bytes())
        .expect("Failed to decode base64 string");

    let section =
        parse_splice_info_section(&buffer).expect("Failed to parse time_signal with descriptors");

    // Validate header
    assert_eq!(section.table_id, 0xFC);
    assert_eq!(
        section.splice_command_type, 0x06,
        "Command type should be 0x06 (time_signal)"
    );

    // Should have descriptors
    assert!(
        section.descriptor_loop_length > 0,
        "Should have descriptors"
    );
    assert!(
        !section.splice_descriptors.is_empty(),
        "Should have descriptor data"
    );
}

#[test]
#[cfg(feature = "crc-validation")]
fn test_upid_adid_example_invalid_crc() {
    // ADID example with invalid CRC: "/DA4AAAAAAAA///wBQb+AKpFLgAiAiBDVUVJAAAAA3//AAApPWwDDEFCQ0QwMTIzNDU2SHAAAFkTm+A="
    let adid_base64 =
        "/DA4AAAAAAAA///wBQb+AKpFLgAiAiBDVUVJAAAAA3//AAApPWwDDEFCQ0QwMTIzNDU2SHAAAFkTm+A=";
    let buffer = BASE64
        .decode(adid_base64.as_bytes())
        .expect("Failed to decode ADID base64 string");

    // Should fail to parse due to invalid CRC when CRC validation is enabled
    let section = parse_splice_info_section(&buffer);
    assert!(section.is_err());
    let error = section.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("CRC validation failed"));
}

#[test]
#[cfg(not(feature = "crc-validation"))]
fn test_upid_adid_example_no_crc_validation() {
    // ADID example (CRC validation disabled): "/DA4AAAAAAAA///wBQb+AKpFLgAiAiBDVUVJAAAAA3//AAApPWwDDEFCQ0QwMTIzNDU2SHAAAFkTm+A="
    let adid_base64 =
        "/DA4AAAAAAAA///wBQb+AKpFLgAiAiBDVUVJAAAAA3//AAApPWwDDEFCQ0QwMTIzNDU2SHAAAFkTm+A=";
    let buffer = BASE64
        .decode(adid_base64)
        .expect("Failed to decode ADID base64 string");

    // Should parse successfully when CRC validation is disabled
    let section =
        parse_splice_info_section(&buffer).expect("Failed to parse ADID SpliceInfoSection");

    // Validate header
    assert_eq!(section.table_id, 0xFC);
    assert_eq!(
        section.splice_command_type, 0x06,
        "Command type should be 0x06 (time_signal)"
    );

    // Should have descriptors with UPID
    assert!(
        section.descriptor_loop_length > 0,
        "Should have descriptors for UPID"
    );
    assert!(
        !section.splice_descriptors.is_empty(),
        "Should have descriptor data"
    );

    // Check for CUEI descriptor (common in SCTE-35)
    if let Some(first_desc) = section.splice_descriptors.first() {
        assert!(first_desc.length() > 0, "Descriptor should have content");
    }
}

#[test]
fn test_upid_umid_example() {
    // UMID example: "/DBDAAAAAAAA///wBQb+AA2QOQAtAitDVUVJAAAAA3+/BCAwNjBhMmIzNC4wMTAxMDEwNS4wMTAxMGQyMC4xEAEBRKI3vg=="
    let umid_base64 = "/DBDAAAAAAAA///wBQb+AA2QOQAtAitDVUVJAAAAA3+/BCAwNjBhMmIzNC4wMTAxMDEwNS4wMTAxMGQyMC4xEAEBRKI3vg==";
    let buffer = BASE64
        .decode(umid_base64.as_bytes())
        .expect("Failed to decode UMID base64 string");

    let section =
        parse_splice_info_section(&buffer).expect("Failed to parse UMID SpliceInfoSection");

    // Validate header
    assert_eq!(section.table_id, 0xFC);
    assert_eq!(
        section.splice_command_type, 0x06,
        "Command type should be 0x06 (time_signal)"
    );

    // Should have descriptors with UPID
    assert!(
        section.descriptor_loop_length > 0,
        "Should have descriptors for UPID"
    );
    assert!(
        !section.splice_descriptors.is_empty(),
        "Should have descriptor data"
    );
}

#[test]
fn test_upid_isan_example() {
    // ISAN example: "/DA4AAAAAAAA///wBQb+Lom5UgAiAiBDVUVJAAAABn//AAApPWwGDAAAAAA6jQAAAAAAABAAAHGXrpg="
    let isan_base64 =
        "/DA4AAAAAAAA///wBQb+Lom5UgAiAiBDVUVJAAAABn//AAApPWwGDAAAAAA6jQAAAAAAABAAAHGXrpg=";
    let buffer = BASE64
        .decode(isan_base64.as_bytes())
        .expect("Failed to decode ISAN base64 string");

    let section =
        parse_splice_info_section(&buffer).expect("Failed to parse ISAN SpliceInfoSection");

    // Validate header
    assert_eq!(section.table_id, 0xFC);
    assert_eq!(
        section.splice_command_type, 0x06,
        "Command type should be 0x06 (time_signal)"
    );

    // Should have descriptors with UPID
    assert!(
        section.descriptor_loop_length > 0,
        "Should have descriptors for UPID"
    );
    assert!(
        !section.splice_descriptors.is_empty(),
        "Should have descriptor data"
    );
}

#[test]
fn test_upid_airid_example() {
    // AIRID example: "/DBhAAAAAAAA///wBQb+qM1E7QBLAhdDVUVJSAAArX+fCAgAAAAALLLXnTUCAAIXQ1VFSUgAACZ/nwgIAAAAACyy150RAAACF0NVRUlIAAAnf58ICAAAAAAsstezEAAAihiGnw=="
    let airid_base64 = "/DBhAAAAAAAA///wBQb+qM1E7QBLAhdDVUVJSAAArX+fCAgAAAAALLLXnTUCAAIXQ1VFSUgAACZ/nwgIAAAAACyy150RAAACF0NVRUlIAAAnf58ICAAAAAAsstezEAAAihiGnw==";
    let buffer = BASE64
        .decode(airid_base64.as_bytes())
        .expect("Failed to decode AIRID base64 string");

    let section =
        parse_splice_info_section(&buffer).expect("Failed to parse AIRID SpliceInfoSection");

    // Validate header
    assert_eq!(section.table_id, 0xFC);
    assert_eq!(
        section.splice_command_type, 0x06,
        "Command type should be 0x06 (time_signal)"
    );

    // Should have multiple descriptors
    assert!(
        section.descriptor_loop_length > 0,
        "Should have descriptors for UPID"
    );
    assert!(
        !section.splice_descriptors.is_empty(),
        "Should have descriptor data"
    );
    assert!(
        section.splice_descriptors.len() >= 3,
        "Should have multiple descriptors"
    );
}

#[test]
fn test_time_signal_placement_opportunity_end() {
    // Time Signal - Placement Opportunity End example
    let placement_end_base64 =
        "/DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=";
    let buffer = BASE64
        .decode(placement_end_base64.as_bytes())
        .expect("Failed to decode placement opportunity end base64 string");

    let section = parse_splice_info_section(&buffer)
        .expect("Failed to parse placement opportunity end SpliceInfoSection");

    // Validate header
    assert_eq!(section.table_id, 0xFC, "Table ID should be 0xFC");
    assert_eq!(
        section.splice_command_type, 0x06,
        "Command type should be 0x06 (time_signal)"
    );

    // Validate command
    match section.splice_command {
        SpliceCommand::TimeSignal(ref cmd) => {
            assert_eq!(
                cmd.splice_time.time_specified_flag, 1,
                "Time should be specified"
            );
            assert!(
                cmd.splice_time.pts_time.is_some(),
                "PTS time should be present"
            );

            // Verify time conversion
            if let Some(duration) = cmd.splice_time.to_duration() {
                // This should represent the end of a placement opportunity
                assert!(duration.as_secs() > 0, "Duration should be positive");
            }
        }
        _ => panic!("Expected TimeSignal command"),
    }

    // Should have descriptors indicating placement opportunity end
    assert!(
        section.descriptor_loop_length > 0,
        "Should have descriptors for placement opportunity end"
    );
    assert!(
        !section.splice_descriptors.is_empty(),
        "Should have descriptor data"
    );

    // Check for segmentation descriptor (common for placement opportunities)
    if let Some(first_desc) = section.splice_descriptors.first() {
        assert!(first_desc.length() > 0, "Descriptor should have content");
        // Descriptor tag 2 is typically segmentation_descriptor
        assert_eq!(first_desc.tag(), 2, "Should be segmentation descriptor");
    }
}

#[test]
fn test_multiple_descriptor_types() {
    // Test that we can parse messages with different types of descriptors
    // This demonstrates our parser can handle various SCTE-35 message formats

    // Test 1: Simple time signal (already covered above)
    let time_signal_base64 = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
    let buffer = BASE64.decode(time_signal_base64.as_bytes()).unwrap();
    let section = parse_splice_info_section(&buffer).unwrap();
    assert_eq!(section.splice_command_type, 0x06);

    // Test 2: Time signal with descriptors (already covered above)
    let time_signal_desc_base64 = "/DAgAAAAAAAAAP/wBQb+Qjo1vQAKAAhDVUVJAAAE0iVuWvA=";
    let buffer2 = BASE64.decode(time_signal_desc_base64.as_bytes()).unwrap();
    let section2 = parse_splice_info_section(&buffer2).unwrap();
    assert_eq!(section2.splice_command_type, 0x06);
    assert!(section2.descriptor_loop_length > 0);

    // Test 3: Complex message with multiple descriptors (AIRID example already covered)
    let complex_base64 = "/DBhAAAAAAAA///wBQb+qM1E7QBLAhdDVUVJSAAArX+fCAgAAAAALLLXnTUCAAIXQ1VFSUgAACZ/nwgIAAAAACyy150RAAACF0NVRUlIAAAnf58ICAAAAAAsstezEAAAihiGnw==";
    let buffer3 = BASE64.decode(complex_base64.as_bytes()).unwrap();
    let section3 = parse_splice_info_section(&buffer3).unwrap();
    assert_eq!(section3.splice_command_type, 0x06);
    assert!(section3.splice_descriptors.len() >= 3);
}

#[test]
fn test_duration_conversions() {
    // Test BreakDuration conversion
    let break_duration = BreakDuration {
        auto_return: 1,
        reserved: 0,
        duration: 5_427_000, // 60.3 seconds in 90kHz ticks
    };

    let duration: Duration = break_duration.to_duration();
    assert_eq!(duration.as_secs(), 60);
    assert_eq!(duration.subsec_millis(), 300);

    // Test using Into trait
    let break_duration2 = BreakDuration {
        auto_return: 1,
        reserved: 0,
        duration: 90_000, // Exactly 1 second
    };

    let duration2: Duration = break_duration2.into();
    assert_eq!(duration2.as_secs(), 1);
    assert_eq!(duration2.subsec_nanos(), 0);

    // Test reference conversion
    let break_duration3 = BreakDuration {
        auto_return: 1,
        reserved: 0,
        duration: 45_000, // 0.5 seconds
    };

    let duration3: Duration = (&break_duration3).into();
    assert_eq!(duration3.as_secs(), 0);
    assert_eq!(duration3.subsec_millis(), 500);

    // Test SpliceTime conversion
    let splice_time = SpliceTime {
        time_specified_flag: 1,
        pts_time: Some(1_935_360_000), // 21504 seconds
    };

    let duration4 = splice_time.to_duration().unwrap();
    assert_eq!(duration4.as_secs(), 21504);
    assert_eq!(duration4.subsec_nanos(), 0);

    // Test SpliceTime with None
    let splice_time_none = SpliceTime {
        time_specified_flag: 0,
        pts_time: None,
    };

    assert!(splice_time_none.to_duration().is_none());
}

#[test]
fn test_splice_descriptor_as_str() {
    // Test with valid UTF-8 bytes
    let descriptor = SpliceDescriptor::Unknown {
        tag: 0x00,
        length: 5,
        data: vec![0x48, 0x65, 0x6c, 0x6c, 0x6f], // "Hello"
    };

    assert_eq!(descriptor.as_str(), Some("Hello".to_string()));

    // Test with invalid UTF-8 bytes
    let invalid_descriptor = SpliceDescriptor::Unknown {
        tag: 0x00,
        length: 3,
        data: vec![0xff, 0xfe, 0xfd], // Invalid UTF-8
    };

    assert_eq!(invalid_descriptor.as_str(), None);

    // Test with empty bytes
    let empty_descriptor = SpliceDescriptor::Unknown {
        tag: 0x00,
        length: 0,
        data: vec![],
    };

    assert_eq!(empty_descriptor.as_str(), Some("".to_string()));
}

#[test]
fn test_parse_splice_info_section() {
    let example_buffer_base64 =
        "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=";
    let example_buffer = BASE64
        .decode(example_buffer_base64.as_bytes())
        .expect("Failed to decode base64 string");

    let section =
        parse_splice_info_section(&example_buffer).expect("Failed to parse SpliceInfoSection");

    // Validate header fields
    assert_eq!(section.table_id, 0xFC, "Table ID should be 0xFC");
    assert_eq!(
        section.section_syntax_indicator, 0,
        "Section syntax indicator should be 0 (MPEG Short Section)"
    );
    assert_eq!(
        section.private_indicator, 0,
        "Private indicator should be 0 (Not Private)"
    );
    assert_eq!(section.section_length, 47, "Section length should be 47");
    assert_eq!(section.protocol_version, 0, "Protocol version should be 0");
    assert_eq!(
        section.encrypted_packet, 0,
        "Encrypted packet should be 0 (unencrypted)"
    );
    assert_eq!(
        section.pts_adjustment, 0x000000000,
        "PTS adjustment should be 0x000000000"
    );
    assert_eq!(section.tier, 0xfff, "Tier should be 0xfff");

    // Validate splice command fields
    assert_eq!(
        section.splice_command_length, 0x14,
        "Splice command length should be 0x14"
    );
    assert_eq!(
        section.splice_command_type, 0x05,
        "Splice command type should be 0x05 (SpliceInsert)"
    );

    // Validate SpliceInsert command specifics
    match section.splice_command {
        SpliceCommand::SpliceInsert(ref cmd) => {
            assert_eq!(
                cmd.splice_event_id, 0x4800008f,
                "Splice Event ID should be 0x4800008f"
            );
            assert_eq!(
                cmd.out_of_network_indicator, 1,
                "Out of network indicator should be 1"
            );
            assert_eq!(
                cmd.program_splice_flag, 1,
                "Program splice flag should be 1"
            );
            assert_eq!(cmd.duration_flag, 1, "Duration flag should be 1");
            assert_eq!(
                cmd.splice_immediate_flag, 0,
                "Splice immediate flag should be 0"
            );

            // Check splice time
            assert!(cmd.splice_time.is_some(), "Splice time should be present");
            if let Some(splice_time) = &cmd.splice_time {
                assert_eq!(
                    splice_time.time_specified_flag, 1,
                    "Time specified flag should be 1"
                );
                assert_eq!(
                    splice_time.pts_time,
                    Some(0x07369c02e),
                    "PTS time should be 0x07369c02e"
                );
            }

            // Check break duration
            assert!(
                cmd.break_duration.is_some(),
                "Break duration should be present"
            );
            if let Some(break_duration) = &cmd.break_duration {
                assert_eq!(break_duration.auto_return, 1, "Auto return should be 1");
                assert_eq!(
                    break_duration.duration, 0x00052ccf5,
                    "Duration should be 0x00052ccf5"
                );
            }

            assert_eq!(cmd.unique_program_id, 0, "Unique Program ID should be 0");
            assert_eq!(cmd.avail_num, 0, "Avail Num should be 0");
            assert_eq!(cmd.avails_expected, 0, "Avails Expected should be 0");
        }
        _ => panic!("Expected SpliceInsert command"),
    }

    // Validate descriptor loop
    assert_eq!(
        section.descriptor_loop_length, 10,
        "Descriptor loop length should be 10"
    );
    assert_eq!(
        section.splice_descriptors.len(),
        1,
        "Should have 1 descriptor"
    );

    if let Some(descriptor) = section.splice_descriptors.first() {
        assert_eq!(
            descriptor.tag(),
            0x00,
            "Descriptor tag should be 0x00 (Avail Descriptor)"
        );
        assert_eq!(descriptor.length(), 8, "Descriptor length should be 8");

        // For unknown descriptors, validate the raw bytes
        if let Some(raw_bytes) = descriptor.raw_bytes() {
            // Validate avail descriptor identifier (first 4 bytes should be 0x00000135)
            assert_eq!(raw_bytes[0], 0x43, "First byte should be 0x43");
            assert_eq!(raw_bytes[1], 0x55, "Second byte should be 0x55");
            assert_eq!(raw_bytes[2], 0x45, "Third byte should be 0x45");
            assert_eq!(raw_bytes[3], 0x49, "Fourth byte should be 0x49");
            assert_eq!(raw_bytes[4], 0x00, "Fifth byte should be 0x00");
            assert_eq!(raw_bytes[5], 0x00, "Sixth byte should be 0x00");
            assert_eq!(raw_bytes[6], 0x01, "Seventh byte should be 0x01");
            assert_eq!(raw_bytes[7], 0x35, "Eighth byte should be 0x35");
        }
    }
}

#[test]
#[cfg(feature = "crc-validation")]
fn test_valid_crc() {
    let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
    let buffer = BASE64.decode(valid_message.as_bytes()).unwrap();

    let result = validate_scte35_crc(&buffer);
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
#[cfg(feature = "crc-validation")]
fn test_invalid_crc() {
    let mut buffer = BASE64
        .decode("/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==".as_bytes())
        .unwrap();

    // Corrupt the CRC (last 4 bytes)
    let len = buffer.len();
    buffer[len - 1] = 0x00;

    let result = validate_scte35_crc(&buffer);
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
#[cfg(feature = "crc-validation")]
fn test_parse_with_crc_validation() {
    let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
    let buffer = BASE64.decode(valid_message.as_bytes()).unwrap();

    // Should parse successfully with valid CRC
    let section = parse_splice_info_section(&buffer);
    assert!(section.is_ok());
}

#[test]
#[cfg(feature = "crc-validation")]
fn test_parse_with_invalid_crc_fails() {
    let mut buffer = BASE64
        .decode("/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==".as_bytes())
        .unwrap();

    // Corrupt the CRC (last 4 bytes)
    let len = buffer.len();
    buffer[len - 1] = 0x00;

    // Should fail to parse with invalid CRC
    let section = parse_splice_info_section(&buffer);
    assert!(section.is_err());
    let error = section.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("CRC validation failed"));
}

#[test]
#[cfg(feature = "crc-validation")]
fn test_splice_info_section_validate_crc() {
    let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
    let buffer = BASE64.decode(valid_message.as_bytes()).unwrap();

    let section = parse_splice_info_section(&buffer).unwrap();

    // Test method-based validation
    let result = section.validate_crc(&buffer);
    assert!(result.is_ok());
    assert!(result.unwrap());

    // Test get_crc method
    assert_eq!(section.get_crc(), section.crc_32);
}

#[test]
#[cfg(feature = "crc-validation")]
fn test_crc_validatable_trait() {
    let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
    let buffer = BASE64.decode(valid_message.as_bytes()).unwrap();

    let section = parse_splice_info_section(&buffer).unwrap();

    // Test trait implementation
    let result = section.validate_crc(&buffer);
    assert!(result.is_ok());
    assert!(result.unwrap());

    let crc = section.get_crc();
    assert!(crc > 0);
}

#[test]
#[cfg(not(feature = "crc-validation"))]
fn test_crc_disabled() {
    let valid_message = "/DAWAAAAAAAAAP/wBQb+Qjo1vQAAuwxz9A==";
    let buffer = BASE64.decode(valid_message).unwrap();

    // Should always return false when CRC validation is disabled
    let result = validate_scte35_crc(&buffer);
    assert!(result.is_ok());
    assert!(!result.unwrap());

    // Parse should still work without CRC validation
    let section = parse_splice_info_section(&buffer);
    assert!(section.is_ok());

    // Method should return false when disabled
    let section = section.unwrap();
    let result = section.validate_crc(&buffer);
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_segmentation_upid_type_conversions() {
    // Test From<u8> implementation
    assert_eq!(
        SegmentationUpidType::from(0x00),
        SegmentationUpidType::NotUsed
    );
    assert_eq!(SegmentationUpidType::from(0x03), SegmentationUpidType::AdID);
    assert_eq!(SegmentationUpidType::from(0x04), SegmentationUpidType::UMID);
    assert_eq!(SegmentationUpidType::from(0x06), SegmentationUpidType::ISAN);
    assert_eq!(
        SegmentationUpidType::from(0x08),
        SegmentationUpidType::AiringID
    );
    assert_eq!(SegmentationUpidType::from(0x0C), SegmentationUpidType::MPU);
    assert_eq!(SegmentationUpidType::from(0x10), SegmentationUpidType::UUID);
    assert_eq!(SegmentationUpidType::from(0x11), SegmentationUpidType::SCR);

    // Test reserved values
    assert_eq!(
        SegmentationUpidType::from(0x50),
        SegmentationUpidType::Reserved(0x50)
    );
    assert_eq!(
        SegmentationUpidType::from(0xFF),
        SegmentationUpidType::Reserved(0xFF)
    );

    // Test Into<u8> implementation (From<SegmentationUpidType> for u8)
    assert_eq!(u8::from(SegmentationUpidType::NotUsed), 0x00);
    assert_eq!(u8::from(SegmentationUpidType::AdID), 0x03);
    assert_eq!(u8::from(SegmentationUpidType::UMID), 0x04);
    assert_eq!(u8::from(SegmentationUpidType::ISAN), 0x06);
    assert_eq!(u8::from(SegmentationUpidType::AiringID), 0x08);
    assert_eq!(u8::from(SegmentationUpidType::MPU), 0x0C);
    assert_eq!(u8::from(SegmentationUpidType::UUID), 0x10);
    assert_eq!(u8::from(SegmentationUpidType::SCR), 0x11);
    assert_eq!(u8::from(SegmentationUpidType::Reserved(0x99)), 0x99);
}

#[test]
fn test_segmentation_upid_type_descriptions() {
    assert_eq!(SegmentationUpidType::NotUsed.to_string(), "Not Used");
    assert_eq!(
        SegmentationUpidType::UserDefinedDeprecated.to_string(),
        "User Defined (Deprecated)"
    );
    assert_eq!(
        SegmentationUpidType::ISCI.to_string(),
        "ISCI (Industry Standard Commercial Identifier)"
    );
    assert_eq!(SegmentationUpidType::AdID.to_string(), "Ad Identifier");
    assert_eq!(
        SegmentationUpidType::UMID.to_string(),
        "UMID (Unique Material Identifier)"
    );
    assert_eq!(
        SegmentationUpidType::ISANDeprecated.to_string(),
        "ISAN (Deprecated)"
    );
    assert_eq!(
        SegmentationUpidType::ISAN.to_string(),
        "ISAN (International Standard Audiovisual Number)"
    );
    assert_eq!(
        SegmentationUpidType::TID.to_string(),
        "TID (Turner Identifier)"
    );
    assert_eq!(SegmentationUpidType::AiringID.to_string(), "Airing ID");
    assert_eq!(
        SegmentationUpidType::ADI.to_string(),
        "ADI (Advertising Digital Identification)"
    );
    assert_eq!(
        SegmentationUpidType::EIDR.to_string(),
        "EIDR (Entertainment Identifier Registry)"
    );
    assert_eq!(
        SegmentationUpidType::ATSCContentIdentifier.to_string(),
        "ATSC Content Identifier"
    );
    assert_eq!(
        SegmentationUpidType::MPU.to_string(),
        "MPU (Media Processing Unit)"
    );
    assert_eq!(
        SegmentationUpidType::MID.to_string(),
        "MID (Media Identifier)"
    );
    assert_eq!(
        SegmentationUpidType::ADSInformation.to_string(),
        "ADS Information"
    );
    assert_eq!(
        SegmentationUpidType::URI.to_string(),
        "URI (Uniform Resource Identifier)"
    );
    assert_eq!(
        SegmentationUpidType::UUID.to_string(),
        "UUID (Universally Unique Identifier)"
    );
    assert_eq!(
        SegmentationUpidType::SCR.to_string(),
        "SCR (Subscriber Company Reporting)"
    );
    assert_eq!(
        SegmentationUpidType::Reserved(0x99).to_string(),
        "Reserved/Unknown"
    );
}

#[test]
fn test_segmentation_upid_type_default() {
    assert_eq!(
        SegmentationUpidType::default(),
        SegmentationUpidType::NotUsed
    );
}

#[test]
fn test_segmentation_upid_type_roundtrip() {
    // Test that all defined types can round-trip through u8 conversion
    let types = [
        SegmentationUpidType::NotUsed,
        SegmentationUpidType::UserDefinedDeprecated,
        SegmentationUpidType::ISCI,
        SegmentationUpidType::AdID,
        SegmentationUpidType::UMID,
        SegmentationUpidType::ISANDeprecated,
        SegmentationUpidType::ISAN,
        SegmentationUpidType::TID,
        SegmentationUpidType::AiringID,
        SegmentationUpidType::ADI,
        SegmentationUpidType::EIDR,
        SegmentationUpidType::ATSCContentIdentifier,
        SegmentationUpidType::MPU,
        SegmentationUpidType::MID,
        SegmentationUpidType::ADSInformation,
        SegmentationUpidType::URI,
        SegmentationUpidType::UUID,
        SegmentationUpidType::SCR,
        SegmentationUpidType::Reserved(0x50),
    ];

    for upid_type in types {
        let byte_value = u8::from(upid_type);
        let back_to_type = SegmentationUpidType::from(byte_value);
        assert_eq!(
            upid_type, back_to_type,
            "Round-trip failed for {:?}",
            upid_type
        );
    }
}

#[test]
fn test_segmentation_descriptor_upid_as_string() {
    // Test AdID (text-based UPID)
    let ad_id_descriptor = SegmentationDescriptor {
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
        segmentation_upid_type: SegmentationUpidType::AdID,
        segmentation_upid_length: 12,
        segmentation_upid: b"ABCD01234567".to_vec(),
        segmentation_type_id: 0x30,
        segmentation_type: SegmentationType::from_id(0x30),
        segment_num: 1,
        segments_expected: 1,
        sub_segment_num: None,
        sub_segments_expected: None,
    };

    assert_eq!(
        ad_id_descriptor.upid_as_string(),
        Some("ABCD01234567".to_string())
    );

    // Test UUID (16-byte format)
    let uuid_bytes = vec![
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde,
        0xf0,
    ];
    let uuid_descriptor = SegmentationDescriptor {
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
        segmentation_upid_type: SegmentationUpidType::UUID,
        segmentation_upid_length: 16,
        segmentation_upid: uuid_bytes,
        segmentation_type_id: 0x30,
        segmentation_type: SegmentationType::from_id(0x30),
        segment_num: 1,
        segments_expected: 1,
        sub_segment_num: None,
        sub_segments_expected: None,
    };

    assert_eq!(
        uuid_descriptor.upid_as_string(),
        Some("12345678-9abc-def0-1234-56789abcdef0".to_string())
    );

    // Test ISAN (12-byte format)
    let isan_bytes = vec![
        0x00, 0x00, 0x00, 0x3a, 0x8d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
    ];
    let isan_descriptor = SegmentationDescriptor {
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
        segmentation_upid_type: SegmentationUpidType::ISAN,
        segmentation_upid_length: 12,
        segmentation_upid: isan_bytes,
        segmentation_type_id: 0x30,
        segmentation_type: SegmentationType::from_id(0x30),
        segment_num: 1,
        segments_expected: 1,
        sub_segment_num: None,
        sub_segments_expected: None,
    };

    assert_eq!(
        isan_descriptor.upid_as_string(),
        Some("0000-003a-8d00-0000-0000-1000".to_string())
    );

    // Test unknown UPID type (should return base64)
    let unknown_descriptor = SegmentationDescriptor {
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
        segmentation_upid_type: SegmentationUpidType::Reserved(0x99),
        segmentation_upid_length: 4,
        segmentation_upid: vec![0xDE, 0xAD, 0xBE, 0xEF],
        segmentation_type_id: 0x30,
        segmentation_type: SegmentationType::from_id(0x30),
        segment_num: 1,
        segments_expected: 1,
        sub_segment_num: None,
        sub_segments_expected: None,
    };

    // Should return base64 representation
    assert_eq!(
        unknown_descriptor.upid_as_string(),
        Some("3q2+7w==".to_string())
    );
}

#[test]
fn test_segmentation_descriptor_convenience_methods() {
    let descriptor = SegmentationDescriptor {
        segmentation_event_id: 1,
        segmentation_event_cancel_indicator: false,
        program_segmentation_flag: true,
        segmentation_duration_flag: true,
        delivery_not_restricted_flag: true,
        web_delivery_allowed_flag: None,
        no_regional_blackout_flag: None,
        archive_allowed_flag: None,
        device_restrictions: None,
        segmentation_duration: Some(2_700_000), // 30 seconds in 90kHz ticks
        segmentation_upid_type: SegmentationUpidType::AdID,
        segmentation_upid_length: 12,
        segmentation_upid: b"ABCD01234567".to_vec(),
        segmentation_type_id: 0x30,
        segmentation_type: SegmentationType::from_id(0x30),
        segment_num: 1,
        segments_expected: 1,
        sub_segment_num: None,
        sub_segments_expected: None,
    };

    // Test upid_type_description
    assert_eq!(descriptor.upid_type_description(), "Ad Identifier");

    // Test duration conversion
    let duration = descriptor.duration().unwrap();
    assert_eq!(duration.as_secs(), 30);
    assert_eq!(duration.subsec_nanos(), 0);

    // Test descriptor without duration
    let no_duration_descriptor = SegmentationDescriptor {
        segmentation_duration_flag: false,
        segmentation_duration: None,
        ..descriptor
    };
    assert!(no_duration_descriptor.duration().is_none());
}

#[test]
fn test_format_helper_functions() {
    use crate::upid::{format_base64, format_isan, format_uuid};

    // Test UUID formatting
    let uuid_bytes = vec![
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde,
        0xf0,
    ];
    assert_eq!(
        format_uuid(&uuid_bytes),
        "12345678-9abc-def0-1234-56789abcdef0"
    );

    // Test UUID with wrong length (should fallback to base64)
    let short_uuid = vec![0x12, 0x34];
    assert_eq!(format_uuid(&short_uuid), "EjQ="); // base64 of [0x12, 0x34]

    // Test ISAN formatting
    let isan_bytes = vec![
        0x00, 0x00, 0x00, 0x3a, 0x8d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
    ];
    assert_eq!(format_isan(&isan_bytes), "0000-003a-8d00-0000-0000-1000");

    // Test ISAN with wrong length (should fallback to base64)
    let short_isan = vec![0x12, 0x34];
    assert_eq!(format_isan(&short_isan), "EjQ="); // base64 of [0x12, 0x34]

    // Test base64 formatting
    let test_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
    assert_eq!(format_base64(&test_bytes), "3q2+7w==");
}

#[test]
fn test_segmentation_type_field_populated_during_parsing() {
    // Test that the segmentation_type field is correctly populated from segmentation_type_id during parsing
    let test_message = "/DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=";
    let buffer = BASE64.decode(test_message.as_bytes()).unwrap();

    let section = parse_splice_info_section(&buffer).unwrap();

    // Verify we have a segmentation descriptor
    assert_eq!(section.splice_descriptors.len(), 1);

    match &section.splice_descriptors[0] {
        SpliceDescriptor::Segmentation(seg_desc) => {
            // Verify that the segmentation_type_id was parsed correctly
            assert_eq!(seg_desc.segmentation_type_id, 0x35);

            // Verify that the segmentation_type field was automatically populated from the ID
            assert_eq!(
                seg_desc.segmentation_type,
                SegmentationType::ProviderPlacementOpportunityEnd
            );

            // Verify that the description method works
            assert_eq!(
                seg_desc.segmentation_type_description(),
                "Provider Placement Opportunity End"
            );

            // Verify consistency between the ID and type
            assert_eq!(
                seg_desc.segmentation_type.id(),
                seg_desc.segmentation_type_id
            );
            assert_eq!(
                SegmentationType::from_id(seg_desc.segmentation_type_id),
                seg_desc.segmentation_type
            );
        }
        _ => panic!("Expected a segmentation descriptor"),
    }
}

#[test]
fn test_mpu_upid_example() {
    // Example with MPU UPID type
    let base64_message = "/DAsAAAAAAAAAP/wBQb+7YaD1QAWAhRDVUVJAADc8X+/DAVPVkxZSSIAAJ6Gk2Q=";
    let buffer = BASE64
        .decode(base64_message.as_bytes())
        .expect("Failed to decode base64");

    let section = parse_splice_info_section(&buffer).expect("Failed to parse SCTE-35 message");

    // Verify header
    assert_eq!(section.table_id, 0xFC);
    assert_eq!(section.splice_command_type, 0x06); // TimeSignal

    // Verify TimeSignal command
    match section.splice_command {
        SpliceCommand::TimeSignal(ref cmd) => {
            assert_eq!(cmd.splice_time.time_specified_flag, 1);
            assert_eq!(cmd.splice_time.pts_time, Some(3985015765));
        }
        _ => panic!("Expected TimeSignal command"),
    }

    // Verify segmentation descriptor
    assert_eq!(section.splice_descriptors.len(), 1);
    match &section.splice_descriptors[0] {
        SpliceDescriptor::Segmentation(seg_desc) => {
            assert_eq!(seg_desc.segmentation_event_id, 0x0000dcf1);
            assert!(!seg_desc.segmentation_event_cancel_indicator);
            assert!(seg_desc.program_segmentation_flag);
            assert!(!seg_desc.segmentation_duration_flag);

            // Check UPID type and data
            assert_eq!(seg_desc.segmentation_upid_type, SegmentationUpidType::MPU);
            assert_eq!(seg_desc.segmentation_upid_length, 5);
            assert_eq!(seg_desc.segmentation_upid, b"OVLYI");

            // MPU type should return the string as-is
            assert_eq!(seg_desc.upid_as_string(), Some("OVLYI".to_string()));

            // Check segmentation type
            assert_eq!(seg_desc.segmentation_type_id, 0x22);
            assert_eq!(seg_desc.segmentation_type, SegmentationType::BreakStart);

            assert_eq!(seg_desc.segment_num, 0);
            assert_eq!(seg_desc.segments_expected, 0);
        }
        _ => panic!("Expected segmentation descriptor"),
    }

    // Verify CRC
    assert_eq!(section.crc_32, 0x9E869364);
}
