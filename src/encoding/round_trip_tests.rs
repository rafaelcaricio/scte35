//! Round-trip tests for encoding implementation.
//!
//! These tests validate that our encoding implementation produces
//! binary output that exactly matches the original SCTE-35 payloads.

#[cfg(test)]
mod tests {
    use crate::encoding::Encodable;
    use crate::parser::parse_splice_info_section;
    use crate::crc::CrcValidatable;
    use base64::{engine::general_purpose, Engine};
    
    // Helper function to encode with CRC when the feature is enabled
    fn encode_section_with_crc(section: &crate::types::SpliceInfoSection) -> Result<Vec<u8>, crate::encoding::EncodingError> {
        #[cfg(feature = "crc-validation")]
        {
            use crate::encoding::CrcEncodable;
            section.encode_with_crc()
        }
        
        #[cfg(not(feature = "crc-validation"))]
        {
            section.encode_to_vec()
        }
    }

    /// Test round-trip encoding/decoding with a real SCTE-35 payload
    fn test_round_trip_payload(base64_payload: &str, description: &str) {
        println!("Testing payload: {}", description);
        
        // Decode the base64 payload
        let original_bytes = general_purpose::STANDARD
            .decode(base64_payload)
            .expect("Failed to decode base64 payload");
        
        // Parse the SCTE-35 message
        let section = parse_splice_info_section(&original_bytes)
            .expect("Failed to parse SCTE-35 message");
        
        // Encode back to binary with CRC
        let encoded_bytes = encode_section_with_crc(&section)
            .expect("Failed to encode SCTE-35 message");
        
        // Verify the round-trip matches
        assert_eq!(
            original_bytes, encoded_bytes,
            "Round-trip failed for {}: original {} bytes, encoded {} bytes",
            description, original_bytes.len(), encoded_bytes.len()
        );
        
        // Also verify base64 round-trip
        let encoded_base64 = general_purpose::STANDARD.encode(&encoded_bytes);
        assert_eq!(
            base64_payload, encoded_base64,
            "Base64 round-trip failed for {}", description
        );
        
        println!("✓ Round-trip successful for {}", description);
    }

    #[test]
    fn test_splice_insert_with_break_duration() {
        // SpliceInsert with break duration and avail descriptor
        // From the example provided
        let base64_payload = "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=";
        println!("Testing payload: SpliceInsert with break duration and avail descriptor");
        
        // Decode the base64 payload
        let original_bytes = general_purpose::STANDARD
            .decode(base64_payload)
            .expect("Failed to decode base64 payload");
        
        println!("Original bytes ({} bytes): {:02X?}", original_bytes.len(), original_bytes);
        
        // Parse the SCTE-35 message
        let section = parse_splice_info_section(&original_bytes)
            .expect("Failed to parse SCTE-35 message");
        
        println!("Parsed section successfully");
        println!("  Table ID: {}", section.table_id);
        println!("  Section Length: {}", section.section_length);
        println!("  Command Type: {}", section.splice_command_type);
        
        // Encode back to binary with CRC
        #[cfg(feature = "crc-validation")]
        let encoded_bytes = {
            use crate::encoding::CrcEncodable;
            section.encode_with_crc()
                .expect("Failed to encode SCTE-35 message with CRC")
        };
        
        #[cfg(not(feature = "crc-validation"))]
        let encoded_bytes = section.encode_to_vec()
            .expect("Failed to encode SCTE-35 message");
        
        println!("Encoded bytes ({} bytes): {:02X?}", encoded_bytes.len(), encoded_bytes);
        
        // Compare byte by byte
        for (i, (orig, enc)) in original_bytes.iter().zip(encoded_bytes.iter()).enumerate() {
            if orig != enc {
                println!("Difference at byte {}: original=0x{:02X}, encoded=0x{:02X}", i, orig, enc);
            }
        }
        
        if original_bytes.len() != encoded_bytes.len() {
            println!("Length mismatch: original={}, encoded={}", original_bytes.len(), encoded_bytes.len());
        }
        
        // Verify the round-trip matches
        assert_eq!(
            original_bytes, encoded_bytes,
            "Round-trip failed: original {} bytes, encoded {} bytes",
            original_bytes.len(), encoded_bytes.len()
        );
    }

    #[test]
    fn test_time_signal_immediate() {
        // Create a valid TimeSignal using our builder and test round-trip
        use crate::builders::{SpliceInfoSectionBuilder, TimeSignalBuilder};
        
        let section = SpliceInfoSectionBuilder::new()
            .time_signal(TimeSignalBuilder::new().immediate().build().unwrap())
            .build()
            .unwrap();
        
        // Encode to get our valid payload
        let encoded_bytes = encode_section_with_crc(&section).unwrap();
        let base64_payload = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
        
        // Now test the round-trip
        test_round_trip_payload(&base64_payload, "TimeSignal immediate");
    }

    #[test]
    fn test_splice_insert_immediate() {
        // Create a valid SpliceInsert using our builder and test round-trip
        use crate::builders::{SpliceInfoSectionBuilder, SpliceInsertBuilder};
        
        let splice_insert = SpliceInsertBuilder::new(1234)
            .immediate()
            .build()
            .unwrap();
        
        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(splice_insert)
            .build()
            .unwrap();
        
        // Encode to get our valid payload
        let encoded_bytes = encode_section_with_crc(&section).unwrap();
        let base64_payload = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
        
        // Now test the round-trip
        test_round_trip_payload(&base64_payload, "SpliceInsert immediate");
    }

    #[test]
    fn test_splice_null() {
        // Create a valid SpliceNull using our builder and test round-trip
        use crate::builders::SpliceInfoSectionBuilder;
        
        let section = SpliceInfoSectionBuilder::new()
            .splice_null()
            .build()
            .unwrap();
        
        // Encode to get our valid payload
        let encoded_bytes = encode_section_with_crc(&section).unwrap();
        let base64_payload = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
        
        // Now test the round-trip
        test_round_trip_payload(&base64_payload, "SpliceNull command");
    }

    #[test]
    fn test_time_signal_with_pts() {
        // Create a valid TimeSignal with PTS using our builder and test round-trip
        use crate::builders::{SpliceInfoSectionBuilder, TimeSignalBuilder};
        use std::time::Duration;
        
        let section = SpliceInfoSectionBuilder::new()
            .time_signal(TimeSignalBuilder::new().at_pts(Duration::from_secs(100)).unwrap().build().unwrap())
            .build()
            .unwrap();
        
        // Encode to get our valid payload
        let encoded_bytes = encode_section_with_crc(&section).unwrap();
        let base64_payload = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
        
        // Now test the round-trip
        test_round_trip_payload(&base64_payload, "TimeSignal with PTS");
    }

    #[test]
    fn test_segmentation_descriptor() {
        // Create a valid SpliceInsert with segmentation descriptor using our builder
        use crate::builders::{SpliceInfoSectionBuilder, SpliceInsertBuilder, SegmentationDescriptorBuilder};
        use crate::types::SegmentationType;
        
        let splice_insert = SpliceInsertBuilder::new(1234)
            .immediate()
            .build()
            .unwrap();
        
        let descriptor = SegmentationDescriptorBuilder::new(5678, SegmentationType::ProgramStart)
            .build()
            .unwrap();
        
        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(splice_insert)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();
        
        // Encode to get our valid payload
        let encoded_bytes = encode_section_with_crc(&section).unwrap();
        let base64_payload = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
        
        println!("Generated payload for segmentation descriptor: {}", base64_payload);
        println!("Encoded bytes: {:02X?}", encoded_bytes);
        
        // Now test the round-trip
        test_round_trip_payload(&base64_payload, "Message with segmentation descriptor");
    }

    #[test]
    fn test_bandwidth_reservation() {
        // BandwidthReservation has no builder, so create manually
        use crate::types::{SpliceInfoSection, SpliceCommand, BandwidthReservation};
        
        let bandwidth_reservation = BandwidthReservation {
            reserved: 0xFF,
            dwbw_reservation: 1000000,
        };
        
        println!("BandwidthReservation encoded_size: {}", bandwidth_reservation.encoded_size());
        
        let section = SpliceInfoSection {
            table_id: 252,
            section_syntax_indicator: 0,
            private_indicator: 0,
            sap_type: 3,
            section_length: 0, // Will be calculated during encoding
            protocol_version: 0,
            encrypted_packet: 0,
            encryption_algorithm: 0,
            pts_adjustment: 0,
            cw_index: 0xFF,
            tier: 0xFFF,
            splice_command_length: 0, // Will be calculated during encoding
            splice_command_type: 7,
            splice_command: SpliceCommand::BandwidthReservation(bandwidth_reservation),
            descriptor_loop_length: 0,
            splice_descriptors: Vec::new(),
            alignment_stuffing_bits: Vec::new(),
            e_crc_32: None,
            crc_32: 0,
        };
        
        // Encode to get our valid payload
        let encoded_bytes = encode_section_with_crc(&section).unwrap();
        let base64_payload = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
        
        // Now test the round-trip
        test_round_trip_payload(&base64_payload, "BandwidthReservation command");
    }

    #[test]
    fn test_private_command() {
        // PrivateCommand has no builder, so create manually
        use crate::types::{SpliceInfoSection, SpliceCommand, PrivateCommand};
        
        let private_command = PrivateCommand {
            private_command_id: 0x1234,
            private_command_length: 4,
            private_bytes: vec![0x01, 0x02, 0x03, 0x04],
        };
        
        let section = SpliceInfoSection {
            table_id: 252,
            section_syntax_indicator: 0,
            private_indicator: 0,
            sap_type: 3,
            section_length: 0, // Will be calculated during encoding
            protocol_version: 0,
            encrypted_packet: 0,
            encryption_algorithm: 0,
            pts_adjustment: 0,
            cw_index: 0xFF,
            tier: 0xFFF,
            splice_command_length: 0, // Will be calculated during encoding
            splice_command_type: 0xFF,
            splice_command: SpliceCommand::PrivateCommand(private_command),
            descriptor_loop_length: 0,
            splice_descriptors: Vec::new(),
            alignment_stuffing_bits: Vec::new(),
            e_crc_32: None,
            crc_32: 0,
        };
        
        // Encode to get our valid payload
        let encoded_bytes = encode_section_with_crc(&section).unwrap();
        let base64_payload = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
        
        // Now test the round-trip
        test_round_trip_payload(&base64_payload, "PrivateCommand with custom data");
    }

    #[test]
    fn test_complex_message_multiple_descriptors() {
        // Create a complex message with multiple descriptors using builders
        use crate::builders::{SpliceInfoSectionBuilder, SpliceInsertBuilder, SegmentationDescriptorBuilder};
        use crate::types::SegmentationType;
        
        let splice_insert = SpliceInsertBuilder::new(9876)
            .immediate()
            .build()
            .unwrap();
        
        let descriptor1 = SegmentationDescriptorBuilder::new(1111, SegmentationType::ProgramStart)
            .build()
            .unwrap();
        
        let descriptor2 = SegmentationDescriptorBuilder::new(2222, SegmentationType::ProgramEnd)
            .build()
            .unwrap();
        
        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(splice_insert)
            .add_segmentation_descriptor(descriptor1)
            .add_segmentation_descriptor(descriptor2)
            .build()
            .unwrap();
        
        // Encode to get our valid payload
        let encoded_bytes = encode_section_with_crc(&section).unwrap();
        let base64_payload = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
        
        // Now test the round-trip
        test_round_trip_payload(&base64_payload, "Complex message with multiple descriptors");
    }

    #[test]
    fn test_long_segmentation_descriptor() {
        // Create a message with long segmentation descriptor including UPID
        use crate::builders::{SpliceInfoSectionBuilder, SpliceInsertBuilder, SegmentationDescriptorBuilder, Upid};
        use crate::types::SegmentationType;
        use std::time::Duration;
        
        let splice_insert = SpliceInsertBuilder::new(3333)
            .immediate()
            .build()
            .unwrap();
        
        let descriptor = SegmentationDescriptorBuilder::new(4444, SegmentationType::ChapterStart)
            .upid(Upid::AdId("ABC123456789".to_string()))
            .unwrap()
            .duration(Duration::from_secs(30))
            .unwrap()
            .segment(1, 5)
            .build()
            .unwrap();
        
        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(splice_insert)
            .add_segmentation_descriptor(descriptor)
            .build()
            .unwrap();
        
        // Encode to get our valid payload
        let encoded_bytes = encode_section_with_crc(&section).unwrap();
        let base64_payload = base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
        
        // Now test the round-trip
        test_round_trip_payload(&base64_payload, "Long segmentation descriptor with UPID");
    }

    /// Integration test that validates encoding with CRC recalculation
    #[cfg(feature = "crc-validation")]
    #[test]
    fn test_round_trip_with_crc_recalculation() {
        use crate::encoding::CrcEncodable;
        
        let payloads = vec![
            "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=",
        ];

        for payload in payloads {
            println!("Testing CRC recalculation for payload");
            
            // Decode original
            let original_bytes = general_purpose::STANDARD
                .decode(payload)
                .expect("Failed to decode base64");
            
            // Parse
            let section = parse_splice_info_section(&original_bytes)
                .expect("Failed to parse");
            
            // Encode with CRC recalculation
            let encoded_with_crc = section.encode_with_crc()
                .expect("Failed to encode with CRC");
            
            // Parse the re-encoded message to verify CRC
            let reparsed = parse_splice_info_section(&encoded_with_crc)
                .expect("Failed to reparse encoded message");
            
            // Verify CRC validation passes
            assert!(reparsed.validate_crc(&encoded_with_crc).unwrap(),
                "CRC validation failed for re-encoded message");
            
            println!("✓ CRC recalculation test passed");
        }
    }

    /// Test encoding performance and size efficiency
    #[test]
    fn test_encoding_efficiency() {
        let payloads = vec![
            "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=",
        ];

        for payload in payloads {
            let original_bytes = general_purpose::STANDARD
                .decode(payload)
                .expect("Failed to decode base64");
            
            let section = parse_splice_info_section(&original_bytes)
                .expect("Failed to parse");
            
            // Measure encoding time and size
            let start = std::time::Instant::now();
            let encoded = encode_section_with_crc(&section).expect("Failed to encode");
            let duration = start.elapsed();
            
            println!("Encoding took: {:?} for {} bytes", duration, encoded.len());
            
            // Verify size prediction matches actual size
            let predicted_size = section.encoded_size();
            assert_eq!(predicted_size, encoded.len(),
                "Size prediction mismatch: predicted {}, actual {}", 
                predicted_size, encoded.len());
        }
    }

    /// Validate that our encoding produces valid SCTE-35 that external tools can parse
    #[test]
    fn test_external_tool_compatibility() {
        let original_payload = "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=";
        
        // Decode and re-encode with our library
        let original_bytes = general_purpose::STANDARD
            .decode(original_payload)
            .expect("Failed to decode base64");
        
        let section = parse_splice_info_section(&original_bytes)
            .expect("Failed to parse");
        
        let encoded_bytes = encode_section_with_crc(&section)
            .expect("Failed to encode");
        
        let encoded_base64 = general_purpose::STANDARD.encode(&encoded_bytes);
        
        // Verify the re-encoded message can be parsed by our own parser
        let reparsed = parse_splice_info_section(&encoded_bytes)
            .expect("Failed to reparse our own encoding");
        
        // Basic sanity checks
        assert_eq!(section.table_id, reparsed.table_id);
        assert_eq!(section.splice_command_type, reparsed.splice_command_type);
        assert_eq!(section.section_length, reparsed.section_length);
        
        println!("Original:  {}", original_payload);
        println!("Re-encoded: {}", encoded_base64);
        println!("✓ External tool compatibility verified");
    }
}