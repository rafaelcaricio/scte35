//! Tests for the builder pattern API.

use super::*;
use crate::types::SegmentationType;
use data_encoding::BASE64;
use std::time::Duration;

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    fn test_splice_insert_builder_basic() {
        let splice_insert = SpliceInsertBuilder::new(12345)
            .immediate()
            .duration(Duration::from_secs(30))
            .unique_program_id(0x1234)
            .avail(1, 4)
            .build()
            .unwrap();

        assert_eq!(splice_insert.splice_event_id, 12345);
        assert_eq!(splice_insert.splice_event_cancel_indicator, 0);
        assert_eq!(splice_insert.splice_immediate_flag, 1);
        assert_eq!(splice_insert.duration_flag, 1);
        assert_eq!(splice_insert.unique_program_id, 0x1234);
        assert_eq!(splice_insert.avail_num, 1);
        assert_eq!(splice_insert.avails_expected, 4);
        assert!(splice_insert.break_duration.is_some());
    }

    #[test]
    fn test_splice_insert_builder_with_pts() {
        let splice_insert = SpliceInsertBuilder::new(67890)
            .at_pts(Duration::from_secs(20))
            .unwrap()
            .duration(Duration::from_secs(15))
            .build()
            .unwrap();

        assert_eq!(splice_insert.splice_event_id, 67890);
        assert_eq!(splice_insert.splice_immediate_flag, 0);
        assert!(splice_insert.splice_time.is_some());

        let splice_time = splice_insert.splice_time.unwrap();
        assert_eq!(splice_time.time_specified_flag, 1);
        assert_eq!(splice_time.pts_time, Some(20 * 90_000)); // 20 seconds in 90kHz ticks
    }

    #[test]
    fn test_splice_insert_builder_cancellation() {
        let splice_insert = SpliceInsertBuilder::new(12345)
            .cancel_event()
            .build()
            .unwrap();

        assert_eq!(splice_insert.splice_event_id, 0);
        assert_eq!(splice_insert.splice_event_cancel_indicator, 1);
    }

    #[test]
    fn test_splice_insert_builder_component_splice() {
        let components = vec![
            (0x01, Some(Duration::from_secs(10))), // Video
            (0x02, Some(Duration::from_secs(10))), // Audio 1
            (0x03, None),                          // Audio 2 immediate
        ];

        let splice_insert = SpliceInsertBuilder::new(3333)
            .component_splice(components)
            .unwrap()
            .duration(Duration::from_secs(15))
            .build()
            .unwrap();

        assert_eq!(splice_insert.program_splice_flag, 0);
        assert_eq!(splice_insert.component_count, 3);
        assert_eq!(splice_insert.components.len(), 3);

        // Check first component
        assert_eq!(splice_insert.components[0].component_tag, 0x01);
        assert!(splice_insert.components[0].splice_time.is_some());

        // Check third component (immediate)
        assert_eq!(splice_insert.components[2].component_tag, 0x03);
        assert!(splice_insert.components[2].splice_time.is_some());
    }

    #[test]
    fn test_splice_insert_builder_too_many_components() {
        let components: Vec<_> = (0..=255).map(|i| (i as u8, None)).collect();

        let result = SpliceInsertBuilder::new(1234).component_splice(components);

        assert!(result.is_err());
        match result.unwrap_err() {
            BuilderError::InvalidComponentCount { max, actual } => {
                assert_eq!(max, 255);
                assert_eq!(actual, 256);
            }
            _ => panic!("Expected InvalidComponentCount error"),
        }
    }

    #[test]
    fn test_time_signal_builder_immediate() {
        let time_signal = TimeSignalBuilder::new().immediate().build().unwrap();

        assert_eq!(time_signal.splice_time.time_specified_flag, 0);
        assert_eq!(time_signal.splice_time.pts_time, None);
    }

    #[test]
    fn test_time_signal_builder_with_pts() {
        let time_signal = TimeSignalBuilder::new()
            .at_pts(Duration::from_secs(30))
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(time_signal.splice_time.time_specified_flag, 1);
        assert_eq!(time_signal.splice_time.pts_time, Some(30 * 90_000));
    }

    #[test]
    fn test_segmentation_descriptor_builder_basic() {
        let descriptor = SegmentationDescriptorBuilder::new(5678, SegmentationType::ProgramStart)
            .duration(Duration::from_secs(1800))
            .unwrap()
            .segment(1, 1)
            .build()
            .unwrap();

        assert_eq!(descriptor.segmentation_event_id, 5678);
        assert!(!descriptor.segmentation_event_cancel_indicator);
        assert_eq!(descriptor.segmentation_type, SegmentationType::ProgramStart);
        assert!(descriptor.segmentation_duration_flag);
        assert_eq!(descriptor.segment_num, 1);
        assert_eq!(descriptor.segments_expected, 1);
        assert_eq!(descriptor.segmentation_duration, Some(1800 * 90_000));
    }

    #[test]
    fn test_segmentation_descriptor_builder_with_upid() {
        let descriptor = SegmentationDescriptorBuilder::new(
            7777,
            SegmentationType::DistributorAdvertisementStart,
        )
        .upid(Upid::AdId("ABC123456789".to_string()))
        .unwrap()
        .build()
        .unwrap();

        assert_eq!(
            descriptor.segmentation_upid_type,
            crate::upid::SegmentationUpidType::AdID
        );
        assert_eq!(descriptor.segmentation_upid_length, 12);
        assert_eq!(descriptor.segmentation_upid, b"ABC123456789");
    }

    #[test]
    fn test_segmentation_descriptor_builder_with_uuid() {
        let uuid_bytes = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0,
        ];

        let descriptor =
            SegmentationDescriptorBuilder::new(8888, SegmentationType::ProviderAdvertisementStart)
                .upid(Upid::Uuid(uuid_bytes))
                .unwrap()
                .build()
                .unwrap();

        assert_eq!(
            descriptor.segmentation_upid_type,
            crate::upid::SegmentationUpidType::UUID
        );
        assert_eq!(descriptor.segmentation_upid_length, 16);
        assert_eq!(descriptor.segmentation_upid, uuid_bytes.to_vec());
    }

    #[test]
    fn test_segmentation_descriptor_builder_with_restrictions() {
        let restrictions = DeliveryRestrictions {
            web_delivery_allowed: false,
            no_regional_blackout: false,
            archive_allowed: true,
            device_restrictions: DeviceRestrictions::RestrictGroup1,
        };

        let descriptor = SegmentationDescriptorBuilder::new(
            9999,
            SegmentationType::DistributorAdvertisementStart,
        )
        .delivery_restrictions(restrictions)
        .build()
        .unwrap();

        assert!(!descriptor.delivery_not_restricted_flag);
        assert_eq!(descriptor.web_delivery_allowed_flag, Some(false));
        assert_eq!(descriptor.no_regional_blackout_flag, Some(false));
        assert_eq!(descriptor.archive_allowed_flag, Some(true));
        assert_eq!(descriptor.device_restrictions, Some(1)); // RestrictGroup1
    }

    #[test]
    fn test_segmentation_descriptor_builder_cancellation() {
        let descriptor = SegmentationDescriptorBuilder::new(1111, SegmentationType::ProgramEnd)
            .cancel_event()
            .build()
            .unwrap();

        assert_eq!(descriptor.segmentation_event_id, 0);
        assert!(descriptor.segmentation_event_cancel_indicator);
    }

    #[test]
    fn test_segmentation_descriptor_builder_sub_segments() {
        let descriptor = SegmentationDescriptorBuilder::new(2222, SegmentationType::ChapterStart)
            .segment(3, 10)
            .sub_segment(2, 5)
            .build()
            .unwrap();

        assert_eq!(descriptor.segment_num, 3);
        assert_eq!(descriptor.segments_expected, 10);
        assert_eq!(descriptor.sub_segment_num, Some(2));
        assert_eq!(descriptor.sub_segments_expected, Some(5));
    }

    #[test]
    fn test_segmentation_descriptor_builder_invalid_upid_length() {
        let result = SegmentationDescriptorBuilder::new(1234, SegmentationType::ProgramStart)
            .upid(Upid::AdId("TOOLONG123456789".to_string()));

        assert!(result.is_err());
        match result.unwrap_err() {
            BuilderError::InvalidUpidLength { expected, actual } => {
                assert_eq!(expected, 12);
                assert_eq!(actual, 16);
            }
            _ => panic!("Expected InvalidUpidLength error"),
        }
    }

    #[test]
    fn test_splice_info_section_builder_basic() {
        let splice_insert = SpliceInsertBuilder::new(12345)
            .immediate()
            .duration(Duration::from_secs(30))
            .build()
            .unwrap();

        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(splice_insert)
            .pts_adjustment(0)
            .tier(0x100)
            .build()
            .unwrap();

        assert_eq!(section.table_id, 0xFC);
        assert_eq!(section.protocol_version, 0);
        assert_eq!(section.pts_adjustment, 0);
        assert_eq!(section.tier, 0x100);
        assert_eq!(section.splice_command_type, 0x05); // SpliceInsert
        assert!(matches!(
            section.splice_command,
            crate::types::SpliceCommand::SpliceInsert(_)
        ));
    }

    #[test]
    fn test_splice_info_section_builder_with_descriptors() {
        let descriptor = SegmentationDescriptorBuilder::new(5678, SegmentationType::ProgramStart)
            .duration(Duration::from_secs(1800))
            .unwrap()
            .build()
            .unwrap();

        let time_signal = TimeSignalBuilder::new().immediate().build().unwrap();

        let section = SpliceInfoSectionBuilder::new()
            .time_signal(time_signal)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();

        assert_eq!(section.splice_descriptors.len(), 1);
        assert!(matches!(
            section.splice_descriptors[0],
            crate::descriptors::SpliceDescriptor::Segmentation(_)
        ));
    }

    #[test]
    fn test_splice_info_section_builder_missing_command() {
        let result = SpliceInfoSectionBuilder::new().build();

        assert!(result.is_err());
        match result.unwrap_err() {
            BuilderError::MissingRequiredField(field) => {
                assert_eq!(field, "splice_command");
            }
            _ => panic!("Expected MissingRequiredField error"),
        }
    }

    #[test]
    fn test_splice_null_command() {
        let section = SpliceInfoSectionBuilder::new()
            .splice_null()
            .build()
            .unwrap();

        assert!(matches!(
            section.splice_command,
            crate::types::SpliceCommand::SpliceNull
        ));
        assert_eq!(section.splice_command_type, 0x00);
    }

    #[test]
    fn test_duration_too_large_error() {
        // Test with a duration that exceeds 33-bit PTS limit (0x1_FFFF_FFFF ticks)
        // Max valid duration is (0x1_FFFF_FFFF / 90_000) seconds
        let max_valid_secs = 0x1_FFFF_FFFF / 90_000;
        let huge_duration = Duration::from_secs(max_valid_secs + 1);

        let splice_insert = SpliceInsertBuilder::new(1234)
            .at_pts(huge_duration)
            .unwrap(); // at_pts doesn't validate, build() does

        let result = splice_insert.build();

        assert!(result.is_err());
        match result.unwrap_err() {
            BuilderError::DurationTooLarge { field, duration } => {
                assert_eq!(field, "splice_time");
                assert_eq!(duration, huge_duration);
            }
            _ => panic!("Expected DurationTooLarge error"),
        }
    }

    #[test]
    fn test_pts_adjustment_masking() {
        // Test that PTS adjustment is properly masked to 33 bits
        let section = SpliceInfoSectionBuilder::new()
            .pts_adjustment(0x3_FFFF_FFFF) // More than 33 bits
            .splice_null()
            .build()
            .unwrap();

        assert_eq!(section.pts_adjustment, 0x1_FFFF_FFFF); // Masked to 33 bits
    }

    #[test]
    fn test_tier_masking() {
        // Test that tier is properly masked to 12 bits
        let section = SpliceInfoSectionBuilder::new()
            .tier(0x1FFF) // More than 12 bits
            .splice_null()
            .build()
            .unwrap();

        assert_eq!(section.tier, 0xFFF); // Masked to 12 bits
    }

    #[test]
    fn test_break_duration_builder() {
        let break_duration = BreakDurationBuilder::new(Duration::from_secs(30))
            .auto_return(false)
            .build()
            .unwrap();

        assert_eq!(break_duration.duration, 30 * 90_000);
        assert_eq!(break_duration.auto_return, 0);
        assert_eq!(break_duration.reserved, 0);
    }

    #[test]
    fn test_splice_time_builder() {
        let splice_time = SpliceTimeBuilder::new()
            .at_pts(Duration::from_secs(10))
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(splice_time.time_specified_flag, 1);
        assert_eq!(splice_time.pts_time, Some(10 * 90_000));
    }

    #[test]
    fn test_upid_validation() {
        // Test valid AdID
        let result = SegmentationDescriptorBuilder::new(1234, SegmentationType::ProgramStart)
            .upid(Upid::AdId("ABC123456789".to_string()));
        assert!(result.is_ok());

        // Test invalid AdID length
        let result = SegmentationDescriptorBuilder::new(1234, SegmentationType::ProgramStart)
            .upid(Upid::AdId("SHORT".to_string()));
        assert!(result.is_err());

        // Test valid URI
        let result = SegmentationDescriptorBuilder::new(1234, SegmentationType::ProgramStart)
            .upid(Upid::Uri("https://example.com/content/123".to_string()));
        assert!(result.is_ok());

        // Test empty URI
        let result = SegmentationDescriptorBuilder::new(1234, SegmentationType::ProgramStart)
            .upid(Upid::Uri("".to_string()));
        assert!(result.is_err());
    }

    // Builder Integration Tests - Validate builders can recreate exact SCTE-35 payloads

    #[test]
    fn test_builder_time_signal_with_segmentation_descriptor() {
        // This test validates that our builder can recreate the exact TimeSignal payload:
        // /DAnAAAAAAAAAP/wBQb+AA27oAARAg9DVUVJAAAAAX+HCQA0AAE0xUZn
        //
        // From CLI analysis:
        // - TimeSignal with PTS 900000 (10 seconds in 90kHz ticks)
        // - Segmentation descriptor with:
        //   - Event ID: 0x00000001
        //   - Type: 0x34 (Provider Placement Opportunity Start)
        //   - UPID Type: ADI (0x09) with 0 bytes
        //   - Segment: 0/1

        // Build the time signal command
        let time_signal = TimeSignalBuilder::new()
            .at_pts(std::time::Duration::from_millis(10000)) // 10 seconds = 900000 ticks
            .unwrap()
            .build()
            .unwrap();

        // Build the segmentation descriptor with specific delivery restrictions to match the expected output
        let restrictions = DeliveryRestrictions {
            web_delivery_allowed: false,
            no_regional_blackout: false,
            archive_allowed: true,
            device_restrictions: DeviceRestrictions::RestrictBoth, // 3 in binary = 11
        };

        let descriptor = SegmentationDescriptorBuilder::new(
            1,
            SegmentationType::ProviderPlacementOpportunityStart,
        )
        .upid(Upid::Adi(vec![])) // ADI with 0 bytes
        .unwrap()
        .delivery_restrictions(restrictions)
        .segment(0, 1)
        .build()
        .unwrap();

        // Build the complete message
        let section = SpliceInfoSectionBuilder::new()
            .time_signal(time_signal)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();

        // Encode to base64 using our encoding implementation
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate that our builder-generated payload matches the expected output
        let expected_base64 = "/DAnAAAAAAAAAP/wBQb+AA27oAARAg9DVUVJAAAAAX+HCQA0AAE0xUZn";
        assert_eq!(
            encoded_base64, expected_base64,
            "Builder-generated payload does not match expected SCTE-35 output"
        );

        // Also validate that the generated payload can be round-trip parsed
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields match original
        assert_eq!(reparsed_section.splice_command_type, 0x06); // TimeSignal
        if let crate::types::SpliceCommand::TimeSignal(ts) = &reparsed_section.splice_command {
            assert_eq!(ts.splice_time.pts_time, Some(900000));
        } else {
            panic!("Expected TimeSignal command");
        }

        assert_eq!(reparsed_section.splice_descriptors.len(), 1);
        if let crate::descriptors::SpliceDescriptor::Segmentation(seg) =
            &reparsed_section.splice_descriptors[0]
        {
            assert_eq!(seg.segmentation_event_id, 1);
            assert_eq!(seg.segmentation_type_id, 0x34);
            assert_eq!(seg.segment_num, 0);
            assert_eq!(seg.segments_expected, 1);
        } else {
            panic!("Expected Segmentation descriptor");
        }
    }

    #[test]
    fn test_builder_splice_insert_with_break_duration() {
        // This test validates that our builder can recreate the exact SpliceInsert payload:
        // /DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=
        //
        // From CLI analysis:
        // - SpliceInsert with Event ID 0x4800008f (1207959695)
        // - Out of network: true
        // - PTS time: 0x07369c02e (1936310318 ticks = 21514.559089 seconds)
        // - Break duration: 0x00052ccf5 (5426421 ticks = 60.293567 seconds) with auto return
        // - Avail descriptor with identifier "CUEI"

        use std::time::Duration;

        // Use exact nanosecond values calculated from ticks to avoid rounding errors
        // PTS: 0x07369c02e = 1936310318 ticks -> 21514559088988 nanoseconds (adjusted)
        let pts_duration = Duration::from_nanos(21514559088988);
        // Break duration: 0x00052ccf5 = 5426421 ticks -> 60293566766 nanoseconds (adjusted)
        let break_duration = Duration::from_nanos(60293566766);

        // Build the splice insert command
        let splice_insert = SpliceInsertBuilder::new(0x4800008f)
            .at_pts(pts_duration)
            .unwrap()
            .out_of_network(true)
            .duration(break_duration)
            .auto_return(true)
            .unique_program_id(0)
            .avail(0, 0)
            .build()
            .unwrap();

        // Create the avail descriptor to match the original (from hex analysis)
        let avail_descriptor = crate::descriptors::AvailDescriptor {
            identifier: 0x43554549,                          // "CUEI"
            provider_avail_id: vec![0x00, 0x00, 0x01, 0x35], // Exact bytes from original payload at offset 42
        };

        // Build the complete message with avail descriptor
        let section = SpliceInfoSectionBuilder::new()
            .cw_index(0xFF) // Set cw_index to match expected payload
            .splice_insert(splice_insert)
            .add_descriptor(crate::descriptors::SpliceDescriptor::Avail(
                avail_descriptor,
            ))
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate that our builder-generated payload matches the expected output
        let expected_base64 =
            "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=";
        assert_eq!(
            encoded_base64, expected_base64,
            "Builder-generated payload does not match expected SCTE-35 output"
        );

        // Also validate round-trip parsing
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x05); // SpliceInsert
        if let crate::types::SpliceCommand::SpliceInsert(si) = &reparsed_section.splice_command {
            assert_eq!(si.splice_event_id, 0x4800008f);
            assert_eq!(si.out_of_network_indicator, 1);
            assert!(si.splice_time.is_some());
            assert!(si.break_duration.is_some());
        } else {
            panic!("Expected SpliceInsert command");
        }
    }

    #[test]
    fn test_builder_time_signal_immediate() {
        // This test validates that our builder can create a time signal with immediate flag
        // TimeSignal immediate mode means the splice should happen immediately (no PTS time)

        use crate::builders::TimeSignalBuilder;

        // Build the time signal command in immediate mode
        let time_signal = TimeSignalBuilder::new().immediate().build().unwrap();

        // Build the complete message
        let section = SpliceInfoSectionBuilder::new()
            .time_signal(time_signal)
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate round-trip parsing works
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x06); // TimeSignal
        if let crate::types::SpliceCommand::TimeSignal(ts) = &reparsed_section.splice_command {
            assert_eq!(
                ts.splice_time.time_specified_flag, 0,
                "Time signal should be immediate (time_specified_flag = 0)"
            );
            assert!(
                ts.splice_time.pts_time.is_none(),
                "Time signal should be immediate (no pts_time)"
            );
        } else {
            panic!("Expected TimeSignal command");
        }

        // Print the generated payload for verification
        println!("Generated TimeSignal immediate payload: {}", encoded_base64);
    }

    #[test]
    fn test_builder_splice_insert_immediate() {
        // This test validates that our builder can create a splice insert with immediate flag
        // SpliceInsert immediate mode means the splice should happen immediately (no PTS time)

        use crate::builders::SpliceInsertBuilder;

        // Build the splice insert command in immediate mode (no specific event ID for testing)
        let splice_insert = SpliceInsertBuilder::new(0x1234567)
            .immediate()
            .out_of_network(true)
            .build()
            .unwrap();

        // Build the complete message
        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(splice_insert)
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate round-trip parsing works
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x05); // SpliceInsert
        if let crate::types::SpliceCommand::SpliceInsert(si) = &reparsed_section.splice_command {
            assert_eq!(si.splice_event_id, 0x1234567);
            assert_eq!(si.out_of_network_indicator, 1);
            assert_eq!(
                si.splice_immediate_flag, 1,
                "SpliceInsert should be immediate"
            );
            assert!(
                si.splice_time.is_none(),
                "SpliceInsert should be immediate (no splice_time)"
            );
        } else {
            panic!("Expected SpliceInsert command");
        }

        // Print the generated payload for verification
        println!(
            "Generated SpliceInsert immediate payload: {}",
            encoded_base64
        );
    }

    #[test]
    fn test_builder_splice_null() {
        // This test validates that our builder can create a splice null command
        // SpliceNull is used for maintaining timing in the message stream

        // Build the complete message with splice null
        let section = SpliceInfoSectionBuilder::new()
            .splice_null()
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate round-trip parsing works
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x00); // SpliceNull
        if let crate::types::SpliceCommand::SpliceNull = &reparsed_section.splice_command {
            // SpliceNull has no additional fields to verify
        } else {
            panic!("Expected SpliceNull command");
        }

        // Print the generated payload for verification
        println!("Generated SpliceNull payload: {}", encoded_base64);
    }

    #[test]
    fn test_builder_time_signal_with_pts() {
        // This test validates that our builder can create a time signal with PTS time
        // TimeSignal with PTS specifies when the splice should occur

        use crate::builders::TimeSignalBuilder;
        use std::time::Duration;

        // Build the time signal command with a specific PTS time (5 seconds)
        let time_signal = TimeSignalBuilder::new()
            .at_pts(Duration::from_secs(5))
            .unwrap()
            .build()
            .unwrap();

        // Build the complete message
        let section = SpliceInfoSectionBuilder::new()
            .time_signal(time_signal)
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate round-trip parsing works
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x06); // TimeSignal
        if let crate::types::SpliceCommand::TimeSignal(ts) = &reparsed_section.splice_command {
            assert_eq!(
                ts.splice_time.time_specified_flag, 1,
                "Time signal should have time specified"
            );
            assert!(
                ts.splice_time.pts_time.is_some(),
                "Time signal should have PTS time"
            );
            // 5 seconds = 450000 90kHz ticks
            let expected_ticks = 5 * 90_000;
            assert_eq!(ts.splice_time.pts_time.unwrap(), expected_ticks);
        } else {
            panic!("Expected TimeSignal command");
        }

        // Print the generated payload for verification
        println!("Generated TimeSignal with PTS payload: {}", encoded_base64);
    }

    #[test]
    fn test_builder_segmentation_descriptor() {
        // This test validates that our builder can create a splice insert with segmentation descriptor
        // Combination of SpliceInsertBuilder + SegmentationDescriptorBuilder

        use crate::builders::{SegmentationDescriptorBuilder, SpliceInsertBuilder, Upid};
        use crate::types::SegmentationType;
        use std::time::Duration;

        // Build the splice insert command (event ID for testing)
        let splice_insert = SpliceInsertBuilder::new(0x12345)
            .at_pts(Duration::from_secs(10))
            .unwrap()
            .out_of_network(true)
            .build()
            .unwrap();

        // Build the segmentation descriptor (Chapter Start example)
        let descriptor =
            SegmentationDescriptorBuilder::new(0x12345, SegmentationType::ChapterStart)
                .upid(Upid::AdId("ABC123456789".to_string()))
                .unwrap()
                .duration(Duration::from_secs(300))
                .unwrap()
                .segment(1, 5)
                .build()
                .unwrap();

        // Build the complete message with both splice insert and segmentation descriptor
        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(splice_insert)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate round-trip parsing works
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x05); // SpliceInsert
        if let crate::types::SpliceCommand::SpliceInsert(si) = &reparsed_section.splice_command {
            assert_eq!(si.splice_event_id, 0x12345);
            assert_eq!(si.out_of_network_indicator, 1);
            assert!(si.splice_time.is_some());
        } else {
            panic!("Expected SpliceInsert command");
        }

        // Verify we have descriptors
        assert_eq!(reparsed_section.splice_descriptors.len(), 1);
        if let crate::descriptors::SpliceDescriptor::Segmentation(seg) =
            &reparsed_section.splice_descriptors[0]
        {
            assert_eq!(seg.segmentation_event_id, 0x12345);
            assert_eq!(
                seg.segmentation_type_id,
                SegmentationType::ChapterStart.id()
            );
            assert_eq!(seg.segment_num, 1);
            assert_eq!(seg.segments_expected, 5);
        } else {
            panic!("Expected Segmentation descriptor");
        }

        // Print the generated payload for verification
        println!(
            "Generated SpliceInsert with SegmentationDescriptor payload: {}",
            encoded_base64
        );
    }

    #[test]
    fn test_builder_splice_null_heartbeat() {
        // This test validates that our builder can recreate the exact SpliceNull heartbeat payload:
        // /DARAAAAAAAAAP/wAAAAAHpPv/8=
        // This is a minimal splice null message used for heartbeat purposes

        // Build the complete message with splice null (minimal configuration)
        let section = SpliceInfoSectionBuilder::new()
            .splice_null()
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate that our builder-generated payload matches the expected output
        let expected_base64 = "/DARAAAAAAAAAP/wAAAAAHpPv/8=";
        assert_eq!(
            encoded_base64, expected_base64,
            "Builder-generated SpliceNull payload does not match expected heartbeat message"
        );

        // Also validate round-trip parsing
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x00); // SpliceNull
        assert_eq!(reparsed_section.section_length, 17); // Expected section length
        if let crate::types::SpliceCommand::SpliceNull = &reparsed_section.splice_command {
            // SpliceNull has no additional fields to verify
        } else {
            panic!("Expected SpliceNull command");
        }
    }

    #[test]
    fn test_builder_sample_14_1_time_signal_placement_opportunity_start() {
        // This test validates that our builder can recreate Sample 14.1 from SCTE-35 spec:
        // /DA0AAAAAAAA///wBQb+cr0AUAAeAhxDVUVJSAAAjn/PAAGlmbAICAAAAAAsoKGKNAIAmsnRfg==
        //
        // From CLI analysis:
        // - TimeSignal with PTS: 1924989008 ticks
        // - Segmentation descriptor: Event ID 0x4800008e, Provider PO Start (0x34)
        // - Duration: 307.000 seconds, UPID Type: Airing ID (0x08)
        // - UPID bytes: [0x00, 0x00, 0x00, 0x00, 0x2C, 0xA0, 0xA1, 0x8A]
        // - Segment: 2, Segments Expected: 0

        use crate::builders::{SegmentationDescriptorBuilder, TimeSignalBuilder, Upid};
        use crate::types::SegmentationType;
        use std::time::Duration;

        // Calculate exact Duration from ticks: 1924989008 ticks / 90000 = 21388.76676 seconds
        // Add small adjustment to compensate for precision loss
        let pts_nanos = (1924989008 * 1_000_000_000) / 90_000 + 100;
        let pts_duration = Duration::from_nanos(pts_nanos);

        // Build the time signal command
        let time_signal = TimeSignalBuilder::new()
            .at_pts(pts_duration)
            .unwrap()
            .build()
            .unwrap();

        // Build the segmentation descriptor with Airing ID UPID and delivery restrictions
        // Expected byte 0x20 = 0xcf = 11001111b means:
        // - delivery_not_restricted_flag = false (bit 5 = 0)
        // - web_delivery_allowed = false (bit 4 = 0)
        // - no_regional_blackout = true (bit 3 = 1)
        // - archive_allowed = true (bit 2 = 1)
        // - device_restrictions = 3 (bits 1-0 = 11)
        use crate::builders::{DeliveryRestrictions, DeviceRestrictions};

        let airing_id_bytes = vec![0x00, 0x00, 0x00, 0x00, 0x2C, 0xA0, 0xA1, 0x8A];
        let restrictions = DeliveryRestrictions {
            web_delivery_allowed: false,
            no_regional_blackout: true,
            archive_allowed: true,
            device_restrictions: DeviceRestrictions::RestrictBoth, // = 3
        };

        let descriptor = SegmentationDescriptorBuilder::new(
            0x4800008e,
            SegmentationType::ProviderPlacementOpportunityStart,
        )
        .upid(Upid::AiringId(u64::from_be_bytes([
            airing_id_bytes[0],
            airing_id_bytes[1],
            airing_id_bytes[2],
            airing_id_bytes[3],
            airing_id_bytes[4],
            airing_id_bytes[5],
            airing_id_bytes[6],
            airing_id_bytes[7],
        ])))
        .unwrap()
        .delivery_restrictions(restrictions)
        .duration(Duration::from_secs(307))
        .unwrap()
        .segment(2, 0)
        .build()
        .unwrap();

        // Build the complete message
        let section = SpliceInfoSectionBuilder::new()
            .cw_index(0xFF) // Set cw_index to match expected payload
            .time_signal(time_signal)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate that our builder-generated payload matches the expected SCTE-35 Sample 14.1
        let expected_base64 =
            "/DA0AAAAAAAA///wBQb+cr0AUAAeAhxDVUVJSAAAjn/PAAGlmbAICAAAAAAsoKGKNAIAmsnRfg==";
        assert_eq!(
            encoded_base64, expected_base64,
            "Builder-generated payload does not match SCTE-35 Sample 14.1"
        );

        // Also validate round-trip parsing
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x06); // TimeSignal
        if let crate::types::SpliceCommand::TimeSignal(ts) = &reparsed_section.splice_command {
            assert_eq!(ts.splice_time.pts_time.unwrap(), 1924989008);
        } else {
            panic!("Expected TimeSignal command");
        }
    }

    #[test]
    fn test_builder_sample_14_3_time_signal_placement_opportunity_end() {
        // This test validates that our builder can recreate Sample 14.3 from SCTE-35 spec:
        // /DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=
        //
        // From CLI analysis:
        // - TimeSignal with PTS: 1952616608 ticks
        // - Segmentation descriptor: Event ID 0x4800008e, Provider PO End (0x35)
        // - Duration Flag: false (no duration)
        // - UPID Type: Airing ID (0x08)
        // - UPID bytes: [0x00, 0x00, 0x00, 0x00, 0x2C, 0xA0, 0xA1, 0x8A]
        // - Segment: 2, Segments Expected: 0

        use crate::builders::{
            DeliveryRestrictions, DeviceRestrictions, SegmentationDescriptorBuilder,
            TimeSignalBuilder, Upid,
        };
        use crate::types::SegmentationType;
        use std::time::Duration;

        // Calculate exact Duration from ticks: 1952616608 ticks / 90000
        let pts_nanos = (1952616608 * 1_000_000_000) / 90_000 + 100; // Add precision adjustment
        let pts_duration = Duration::from_nanos(pts_nanos);

        // Build the time signal command
        let time_signal = TimeSignalBuilder::new()
            .at_pts(pts_duration)
            .unwrap()
            .build()
            .unwrap();

        // Build the segmentation descriptor with Airing ID UPID (no duration for End event)
        // Need to check delivery restrictions - let's assume similar to the Start event
        let airing_id_bytes = vec![0x00, 0x00, 0x00, 0x00, 0x2C, 0xA0, 0xA1, 0x8A];
        let restrictions = DeliveryRestrictions {
            web_delivery_allowed: true, // Different from Start event
            no_regional_blackout: true,
            archive_allowed: true,
            device_restrictions: DeviceRestrictions::RestrictBoth,
        };

        let descriptor = SegmentationDescriptorBuilder::new(
            0x4800008e,
            SegmentationType::ProviderPlacementOpportunityEnd,
        )
        .upid(Upid::AiringId(u64::from_be_bytes([
            airing_id_bytes[0],
            airing_id_bytes[1],
            airing_id_bytes[2],
            airing_id_bytes[3],
            airing_id_bytes[4],
            airing_id_bytes[5],
            airing_id_bytes[6],
            airing_id_bytes[7],
        ])))
        .unwrap()
        .delivery_restrictions(restrictions)
        .segment(2, 0)
        .build()
        .unwrap();

        // Build the complete message
        let section = SpliceInfoSectionBuilder::new()
            .cw_index(0xFF) // Set cw_index to match expected payload
            .time_signal(time_signal)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate that our builder-generated payload matches the expected SCTE-35 Sample 14.3
        let expected_base64 =
            "/DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=";
        assert_eq!(
            encoded_base64, expected_base64,
            "Builder-generated payload does not match SCTE-35 Sample 14.3"
        );

        // Also validate round-trip parsing
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x06); // TimeSignal
        if let crate::types::SpliceCommand::TimeSignal(ts) = &reparsed_section.splice_command {
            assert_eq!(ts.splice_time.pts_time.unwrap(), 1952616608);
        } else {
            panic!("Expected TimeSignal command");
        }
    }

    #[test]
    fn test_builder_aws_mediatailor_cue_out() {
        // This test validates a real-world AWS MediaTailor cue-out payload:
        // "/DA9AAAAAAAAAP/wBQb+uYbZqwAnAiVDVUVJAAAKqX//AAEjW4AMEU1EU05CMDAxMTMyMjE5M19ONAAAmXz5JA=="
        //
        // From CLI analysis:
        // - TimeSignal with PTS: 3112622507 ticks
        // - Segmentation descriptor: Event ID 0x00000aa9, Provider PO Start (0x34)
        // - Duration: 212.160 seconds, UPID Type: MPU (0x0c)
        // - UPID: "MDSNB0011322193_N"
        // - Segment: 0, Segments Expected: 0

        use crate::builders::{SegmentationDescriptorBuilder, TimeSignalBuilder, Upid};
        use crate::types::SegmentationType;
        use std::time::Duration;

        // Calculate exact Duration from ticks: 3112622507 ticks / 90000
        let pts_nanos = (3112622507u64 * 1_000_000_000) / 90_000 + 100;
        let pts_duration = Duration::from_nanos(pts_nanos);

        // Build the time signal command
        let time_signal = TimeSignalBuilder::new()
            .at_pts(pts_duration)
            .unwrap()
            .build()
            .unwrap();

        // Build the segmentation descriptor with MPU UPID and duration
        let descriptor = SegmentationDescriptorBuilder::new(
            0x00000aa9,
            SegmentationType::ProviderPlacementOpportunityStart,
        )
        .upid(Upid::Mpu("MDSNB0011322193_N".as_bytes().to_vec()))
        .unwrap()
        .duration(Duration::from_millis(212160)) // 212.160 seconds
        .unwrap()
        .segment(0, 0)
        .build()
        .unwrap();

        // Build the complete message
        let section = SpliceInfoSectionBuilder::new()
            .time_signal(time_signal)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate that our builder-generated payload matches the AWS MediaTailor cue-out
        let expected_base64 = "/DA9AAAAAAAAAP/wBQb+uYbZqwAnAiVDVUVJAAAKqX//AAEjW4AMEU1EU05CMDAxMTMyMjE5M19ONAAAmXz5JA==";
        assert_eq!(
            encoded_base64, expected_base64,
            "Builder-generated payload does not match AWS MediaTailor cue-out"
        );

        // Also validate round-trip parsing
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x06); // TimeSignal
        if let crate::types::SpliceCommand::TimeSignal(ts) = &reparsed_section.splice_command {
            assert_eq!(ts.splice_time.pts_time.unwrap(), 3112622507);
        } else {
            panic!("Expected TimeSignal command");
        }
    }

    #[test]
    fn test_builder_aws_mediatailor_cue_in() {
        // This test validates a real-world AWS MediaTailor cue-in payload:
        // "/DA4AAAAAAAAAP/wBQb+tTeaawAiAiBDVUVJAAAKqH+/DBFNRFNOQjAwMTEzMjIxOTJfTjUAAIiGK1s="
        //
        // From CLI analysis:
        // - TimeSignal with PTS: 3040320107 ticks
        // - Segmentation descriptor: Event ID 0x00000aa8, Provider PO End (0x35)
        // - Duration Flag: false (no duration), UPID Type: MPU (0x0c)
        // - UPID: "MDSNB0011322192_N"
        // - Segment: 0, Segments Expected: 0

        use crate::builders::{SegmentationDescriptorBuilder, TimeSignalBuilder, Upid};
        use crate::types::SegmentationType;
        use std::time::Duration;

        // Calculate exact Duration from ticks: 3040320107 ticks / 90000
        let pts_nanos = (3040320107u64 * 1_000_000_000) / 90_000 + 100;
        let pts_duration = Duration::from_nanos(pts_nanos);

        // Build the time signal command
        let time_signal = TimeSignalBuilder::new()
            .at_pts(pts_duration)
            .unwrap()
            .build()
            .unwrap();

        // Build the segmentation descriptor with MPU UPID (no duration for End event)
        let descriptor = SegmentationDescriptorBuilder::new(
            0x00000aa8,
            SegmentationType::ProviderPlacementOpportunityEnd,
        )
        .upid(Upid::Mpu("MDSNB0011322192_N".as_bytes().to_vec()))
        .unwrap()
        .segment(0, 0)
        .build()
        .unwrap();

        // Build the complete message
        let section = SpliceInfoSectionBuilder::new()
            .time_signal(time_signal)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate that our builder-generated payload matches the AWS MediaTailor cue-in
        let expected_base64 =
            "/DA4AAAAAAAAAP/wBQb+tTeaawAiAiBDVUVJAAAKqH+/DBFNRFNOQjAwMTEzMjIxOTJfTjUAAIiGK1s=";
        assert_eq!(
            encoded_base64, expected_base64,
            "Builder-generated payload does not match AWS MediaTailor cue-in"
        );

        // Also validate round-trip parsing
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x06); // TimeSignal
        if let crate::types::SpliceCommand::TimeSignal(ts) = &reparsed_section.splice_command {
            assert_eq!(ts.splice_time.pts_time.unwrap(), 3040320107);
        } else {
            panic!("Expected TimeSignal command");
        }
    }

    #[test]
    fn test_builder_bitmovin_splice_insert() {
        // This test validates a real-world Bitmovin MPEG-DASH SpliceInsert payload:
        // "/DAlAAAAAAAAAP/wFAUAAAAEf+/+kybGyP4BSvaQAAEBAQAArky/3g=="
        //
        // From CLI analysis:
        // - SpliceInsert with Event ID: 0x00000004
        // - Out of Network: true, PTS: 0x09326c6c8 = 2468792008 ticks (27431.022311 seconds)
        // - Break Duration: 0x0014af690 = 21693072 ticks (241.000000 seconds) with auto return
        // - Unique Program ID: 1, Avail: 1/1
        // - No descriptors

        use crate::builders::SpliceInsertBuilder;
        use std::time::Duration;

        // Calculate exact Duration from ticks using precise arithmetic
        // PTS: 0x09326c6c8 = 2468792008 ticks
        let pts_nanos = (2468792008u64 * 1_000_000_000) / 90_000 + 100; // Add precision adjustment
        let pts_duration = Duration::from_nanos(pts_nanos);

        // Break duration: 0x0014af690 = 21690000 ticks (not 21693072!)
        let break_nanos = (21690000u64 * 1_000_000_000) / 90_000 + 100; // Add precision adjustment
        let break_duration = Duration::from_nanos(break_nanos);

        // Build the splice insert command
        let splice_insert = SpliceInsertBuilder::new(0x00000004)
            .at_pts(pts_duration)
            .unwrap()
            .out_of_network(true)
            .duration(break_duration)
            .auto_return(true)
            .unique_program_id(1)
            .avail(1, 1)
            .build()
            .unwrap();

        // Build the complete message (no descriptors)
        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(splice_insert)
            .build()
            .unwrap();

        // Encode to base64
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section
                .encode_with_crc()
                .expect("Failed to encode with CRC")
        };

        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = {
            use crate::encoding::Encodable;
            section.encode_to_vec().expect("Failed to encode")
        };

        let encoded_base64 = BASE64.encode(&encoded_bytes);

        // Validate that our builder-generated payload matches the Bitmovin SpliceInsert
        let expected_base64 = "/DAlAAAAAAAAAP/wFAUAAAAEf+/+kybGyP4BSvaQAAEBAQAArky/3g==";
        assert_eq!(
            encoded_base64, expected_base64,
            "Builder-generated payload does not match Bitmovin SpliceInsert"
        );

        // Also validate round-trip parsing
        let reparsed_section = crate::parser::parse_splice_info_section(&encoded_bytes)
            .expect("Failed to parse builder-generated payload");

        // Verify key fields
        assert_eq!(reparsed_section.splice_command_type, 0x05); // SpliceInsert
        if let crate::types::SpliceCommand::SpliceInsert(si) = &reparsed_section.splice_command {
            assert_eq!(si.splice_event_id, 0x00000004);
            assert_eq!(si.out_of_network_indicator, 1);
            assert_eq!(si.unique_program_id, 1);
            assert_eq!(si.avail_num, 1);
            assert_eq!(si.avails_expected, 1);
        } else {
            panic!("Expected SpliceInsert command");
        }
    }
}
