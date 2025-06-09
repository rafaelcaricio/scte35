//! Tests for the builder pattern API.

use super::*;
use crate::types::SegmentationType;
use std::time::Duration;

#[cfg(test)]
mod tests {
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
        
        let result = SpliceInsertBuilder::new(1234)
            .component_splice(components);
        
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
        let time_signal = TimeSignalBuilder::new()
            .immediate()
            .build()
            .unwrap();

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
        assert_eq!(descriptor.segmentation_event_cancel_indicator, false);
        assert_eq!(descriptor.segmentation_type, SegmentationType::ProgramStart);
        assert_eq!(descriptor.segmentation_duration_flag, true);
        assert_eq!(descriptor.segment_num, 1);
        assert_eq!(descriptor.segments_expected, 1);
        assert_eq!(descriptor.segmentation_duration, Some(1800 * 90_000));
    }

    #[test]
    fn test_segmentation_descriptor_builder_with_upid() {
        let descriptor = SegmentationDescriptorBuilder::new(7777, SegmentationType::DistributorAdvertisementStart)
            .upid(Upid::AdId("ABC123456789".to_string()))
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(descriptor.segmentation_upid_type, crate::upid::SegmentationUpidType::AdID);
        assert_eq!(descriptor.segmentation_upid_length, 12);
        assert_eq!(descriptor.segmentation_upid, b"ABC123456789");
    }

    #[test]
    fn test_segmentation_descriptor_builder_with_uuid() {
        let uuid_bytes = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
        ];

        let descriptor = SegmentationDescriptorBuilder::new(8888, SegmentationType::ProviderAdvertisementStart)
            .upid(Upid::Uuid(uuid_bytes))
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(descriptor.segmentation_upid_type, crate::upid::SegmentationUpidType::UUID);
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

        let descriptor = SegmentationDescriptorBuilder::new(9999, SegmentationType::DistributorAdvertisementStart)
            .delivery_restrictions(restrictions)
            .build()
            .unwrap();

        assert_eq!(descriptor.delivery_not_restricted_flag, false);
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
        assert_eq!(descriptor.segmentation_event_cancel_indicator, true);
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
        assert!(matches!(section.splice_command, crate::types::SpliceCommand::SpliceInsert(_)));
    }

    #[test]
    fn test_splice_info_section_builder_with_descriptors() {
        let descriptor = SegmentationDescriptorBuilder::new(5678, SegmentationType::ProgramStart)
            .duration(Duration::from_secs(1800))
            .unwrap()
            .build()
            .unwrap();

        let time_signal = TimeSignalBuilder::new()
            .immediate()
            .build()
            .unwrap();

        let section = SpliceInfoSectionBuilder::new()
            .time_signal(time_signal)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();

        assert_eq!(section.splice_descriptors.len(), 1);
        assert!(matches!(section.splice_descriptors[0], crate::descriptors::SpliceDescriptor::Segmentation(_)));
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

        assert!(matches!(section.splice_command, crate::types::SpliceCommand::SpliceNull));
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
    fn test_datetime_builder() {
        let datetime = DateTimeBuilder::new(2023, 12, 25, 14, 30, 45)
            .unwrap()
            .utc(true)
            .build();

        assert_eq!(datetime.year, 2023);
        assert_eq!(datetime.month, 12);
        assert_eq!(datetime.day, 25);
        assert_eq!(datetime.hour, 14);
        assert_eq!(datetime.minute, 30);
        assert_eq!(datetime.second, 45);
        assert_eq!(datetime.utc_flag, 1);
        assert_eq!(datetime.frames, 0);
        assert_eq!(datetime.milliseconds, 0);
    }

    #[test]
    fn test_datetime_builder_validation() {
        // Test invalid month
        let result = DateTimeBuilder::new(2023, 13, 1, 0, 0, 0);
        assert!(result.is_err());

        // Test invalid day
        let result = DateTimeBuilder::new(2023, 1, 32, 0, 0, 0);
        assert!(result.is_err());

        // Test invalid hour
        let result = DateTimeBuilder::new(2023, 1, 1, 24, 0, 0);
        assert!(result.is_err());

        // Test invalid minute
        let result = DateTimeBuilder::new(2023, 1, 1, 0, 60, 0);
        assert!(result.is_err());

        // Test invalid second
        let result = DateTimeBuilder::new(2023, 1, 1, 0, 0, 60);
        assert!(result.is_err());
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
}