/// Demonstrates the SCTE-35 builder pattern API.
///
/// This example shows how to create SCTE-35 messages using the type-safe builder pattern,
/// including splice insert commands, time signals, and segmentation descriptors.
use scte35::builders::*;
use scte35::types::SegmentationType;
use std::time::Duration;

fn main() -> BuilderResult<()> {
    println!("=== SCTE-35 Builder Pattern Demo ===\n");

    // Example 1: Creating a Splice Insert for Ad Break
    println!("1. Creating a 30-second ad break starting at 20 seconds:");

    let splice_insert = SpliceInsertBuilder::new(12345)
        .at_pts(Duration::from_secs(20))?
        .duration(Duration::from_secs(30))
        .unique_program_id(0x1234)
        .avail(1, 4) // First of 4 avails
        .build()?;

    let section = SpliceInfoSectionBuilder::new()
        .pts_adjustment(0)
        .splice_insert(splice_insert)
        .build()?;

    println!(
        "   Event ID: {}",
        section.splice_command.get_event_id().unwrap_or(0)
    );
    println!("   Command Type: 0x{:02x}", section.splice_command_type);
    println!("   Section Length: {} bytes\n", section.section_length);

    // Example 2: Creating a Time Signal with Segmentation Descriptor
    println!("2. Creating a program start boundary with UPID:");

    let segmentation = SegmentationDescriptorBuilder::new(5678, SegmentationType::ProgramStart)
        .upid(Upid::AdId("ABC123456789".to_string()))?
        .duration(Duration::from_secs(1800))? // 30-minute program
        .build()?;

    let section = SpliceInfoSectionBuilder::new()
        .time_signal(TimeSignalBuilder::new().immediate().build()?)
        .add_segmentation_descriptor(segmentation)
        .build()?;

    println!(
        "   Segmentation Event ID: {}",
        section
            .splice_descriptors
            .first()
            .and_then(|d| match d {
                scte35::descriptors::SpliceDescriptor::Segmentation(seg) =>
                    Some(seg.segmentation_event_id),
                _ => None,
            })
            .unwrap_or(0)
    );
    println!("   Descriptors: {}", section.splice_descriptors.len());
    println!(
        "   Descriptor Loop Length: {} bytes\n",
        section.descriptor_loop_length
    );

    // Example 3: Creating an Immediate Splice Out
    println!("3. Creating an immediate splice out to ads:");

    let section = SpliceInfoSectionBuilder::new()
        .splice_insert(
            SpliceInsertBuilder::new(9999)
                .immediate()
                .out_of_network(true)
                .build()?,
        )
        .build()?;

    println!(
        "   Immediate splice: {:?}",
        matches!(section.splice_command, scte35::types::SpliceCommand::SpliceInsert(ref si)
            if si.splice_immediate_flag == 1)
    );
    println!(
        "   Out of network: {:?}",
        matches!(section.splice_command, scte35::types::SpliceCommand::SpliceInsert(ref si)
            if si.out_of_network_indicator == 1)
    );
    println!();

    // Example 4: Component-Level Splice
    println!("4. Creating component-level splice at 10 seconds:");

    let splice_insert = SpliceInsertBuilder::new(3333)
        .component_splice(vec![
            (0x01, Some(Duration::from_secs(10))), // Video component
            (0x02, Some(Duration::from_secs(10))), // Audio component 1
            (0x03, Some(Duration::from_secs(10))), // Audio component 2
        ])?
        .duration(Duration::from_secs(15))
        .build()?;

    let section = SpliceInfoSectionBuilder::new()
        .splice_insert(splice_insert)
        .build()?;

    if let scte35::types::SpliceCommand::SpliceInsert(ref si) = section.splice_command {
        println!("   Program splice: {}", si.program_splice_flag == 1);
        println!("   Component count: {}", si.component_count);
        println!(
            "   Components: {:?}",
            si.components
                .iter()
                .map(|c| c.component_tag)
                .collect::<Vec<_>>()
        );
    }
    println!();

    // Example 5: Complex Segmentation with Delivery Restrictions
    println!("5. Creating segmentation with delivery restrictions:");

    let restrictions = DeliveryRestrictions {
        web_delivery_allowed: false,
        no_regional_blackout: false,
        archive_allowed: true,
        device_restrictions: DeviceRestrictions::RestrictGroup1,
    };

    let uuid_bytes = [
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde,
        0xf0,
    ];

    let segmentation =
        SegmentationDescriptorBuilder::new(7777, SegmentationType::DistributorAdvertisementStart)
            .delivery_restrictions(restrictions)
            .upid(Upid::Uuid(uuid_bytes))?
            .segment(2, 6) // 2nd of 6 segments
            .build()?;

    let section = SpliceInfoSectionBuilder::new()
        .time_signal(
            TimeSignalBuilder::new()
                .at_pts(Duration::from_secs(30))?
                .build()?,
        )
        .add_segmentation_descriptor(segmentation)
        .tier(0x100) // Specific tier
        .build()?;

    println!("   Tier: 0x{:03x}", section.tier);
    println!("   PTS Adjustment: {}", section.pts_adjustment);
    if let Some(scte35::descriptors::SpliceDescriptor::Segmentation(seg)) =
        section.splice_descriptors.first()
    {
        println!(
            "   Delivery restricted: {}",
            !seg.delivery_not_restricted_flag
        );
        println!(
            "   Web delivery allowed: {:?}",
            seg.web_delivery_allowed_flag
        );
        println!("   Segment: {}/{}", seg.segment_num, seg.segments_expected);
    }
    println!();

    // Example 6: Error Handling
    println!("6. Demonstrating error handling:");

    // Try to create an invalid UPID
    let result = SegmentationDescriptorBuilder::new(1234, SegmentationType::ProgramStart)
        .upid(Upid::AdId("TOO_SHORT".to_string()));

    match result {
        Ok(_) => println!("   Unexpected success"),
        Err(BuilderError::InvalidUpidLength { expected, actual }) => {
            println!("   ✓ Caught invalid UPID length: expected {expected}, got {actual}");
        }
        Err(e) => println!("   Unexpected error: {e}"),
    }

    // Try to create a message without a command
    let result = SpliceInfoSectionBuilder::new().build();
    match result {
        Ok(_) => println!("   Unexpected success"),
        Err(BuilderError::MissingRequiredField(field)) => {
            println!("   ✓ Caught missing field: {field}");
        }
        Err(e) => println!("   Unexpected error: {e}"),
    }

    // Example 7: MPU UPID usage patterns
    println!("7. Creating MPU UPIDs for different use cases:");

    // Company-specific format identifier (readable ASCII)
    let company_mpu = Upid::new_mpu_str(0x43554549, "episode_s01e05_12345"); // "CUEI"
    let descriptor1 = SegmentationDescriptorBuilder::new(8888, SegmentationType::ProgramStart)
        .upid(company_mpu.clone())?
        .build()?;
    println!("   ✓ Company MPU created: {}", company_mpu);

    // Different company format with structured data
    let other_mpu = Upid::new_mpu_str(0x4D594944, "show:123:ep:456:segment:789"); // "MYID"
    let _descriptor2 = SegmentationDescriptorBuilder::new(9999, SegmentationType::ContentIdentification)
        .upid(other_mpu.clone())?
        .build()?;
    println!("   ✓ Different company MPU: {}", other_mpu);

    // Binary format identifier with binary data
    let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD, 0xFC];
    let binary_mpu = Upid::new_mpu(0x12345678, binary_data); // Non-ASCII, shows as hex
    let _descriptor3 = SegmentationDescriptorBuilder::new(1010, SegmentationType::ProgramEnd)
        .upid(binary_mpu.clone())?
        .build()?;
    println!("   ✓ Binary format MPU: {}", binary_mpu);

    // Very long content (gets truncated in display)
    let long_content = "this_is_a_very_long_content_identifier_that_exceeds_fifty_characters_and_should_be_truncated";
    let long_mpu = Upid::new_mpu_str(0x4C4F4E47, long_content); // "LONG"
    let _descriptor4 = SegmentationDescriptorBuilder::new(1111, SegmentationType::ProgramStart)
        .upid(long_mpu.clone())?
        .build()?;
    println!("   ✓ Long content MPU: {}", long_mpu);

    // Direct struct construction for full control
    let direct_mpu = Upid::Mpu {
        format_identifier: 0x54455354, // "TEST"
        private_data: vec![0x01, 0x02, 0x03, b'h', b'e', b'l', b'l', b'o'],
    };
    let _descriptor5 = SegmentationDescriptorBuilder::new(1212, SegmentationType::ChapterStart)
        .upid(direct_mpu.clone())?
        .build()?;
    println!("   ✓ Direct construction MPU: {}", direct_mpu);

    // Create a complete message with MPU descriptor
    let time_signal = TimeSignalBuilder::new()
        .at_pts(Duration::from_secs(5))?
        .build()?;
    
    let section = SpliceInfoSectionBuilder::new()
        .time_signal(time_signal)
        .add_segmentation_descriptor(descriptor1)
        .build()?;

    // Show the encoding
    #[cfg(feature = "crc-validation")]
    {
        use scte35::encoding::CrcEncodable;
        match section.encode_with_crc() {
            Ok(encoded) => {
                let base64_encoded = data_encoding::BASE64.encode(&encoded);
                println!("   ✓ Complete message with MPU encoded: {} bytes", encoded.len());
                println!("     Base64: {}...", &base64_encoded[..40]);
            }
            Err(e) => {
                println!("   ! Encoding error: {}", e);
            }
        }
    }

    #[cfg(not(feature = "crc-validation"))]
    {
        use scte35::encoding::Encodable;
        match section.encode_to_vec() {
            Ok(encoded) => {
                let base64_encoded = data_encoding::BASE64.encode(&encoded);
                println!("   ✓ Complete message with MPU encoded: {} bytes", encoded.len());
                println!("     Base64: {}...", &base64_encoded[..40]);
            }
            Err(e) => {
                println!("   ! Encoding error: {}", e);
            }
        }
    }

    println!("\n=== Demo completed successfully! ===");
    Ok(())
}

// Helper trait to extract event ID from SpliceCommand
trait SpliceCommandExt {
    fn get_event_id(&self) -> Option<u32>;
}

impl SpliceCommandExt for scte35::types::SpliceCommand {
    fn get_event_id(&self) -> Option<u32> {
        match self {
            scte35::types::SpliceCommand::SpliceInsert(si) => Some(si.splice_event_id),
            scte35::types::SpliceCommand::SpliceSchedule(ss) => Some(ss.splice_event_id),
            _ => None,
        }
    }
}
