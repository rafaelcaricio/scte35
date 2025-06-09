//! Tests for the encoding module.

#[cfg(test)]
mod encoding_tests {
    use crate::builders::*;
    use crate::crc::CrcValidatable;
    use crate::encoding::{BitWriter, Encodable};
    use crate::time::*;
    use crate::types::*;

    #[test]
    fn test_bit_writer_basic() {
        let mut writer = BitWriter::new();
        writer.write_bits(0xAB, 8).unwrap();
        writer.write_bits(0xCD, 8).unwrap();
        let buffer = writer.finish();
        assert_eq!(buffer, vec![0xAB, 0xCD]);
    }

    #[test]
    fn test_bit_writer_partial_bits() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b101, 3).unwrap();
        writer.write_bits(0b11001, 5).unwrap();
        let buffer = writer.finish();
        // Should produce: 10111001
        assert_eq!(buffer, vec![0b10111001]);
    }

    #[test]
    fn test_splice_time_encoding() {
        let splice_time = SpliceTime {
            time_specified_flag: 1,
            pts_time: Some(0x123456789),
        };

        let mut writer = BitWriter::new();
        splice_time.encode(&mut writer).unwrap();
        let buffer = writer.finish();

        // Verify the encoded size matches expected
        assert_eq!(buffer.len(), splice_time.encoded_size());
    }

    #[test]
    fn test_break_duration_encoding() {
        let break_duration = BreakDuration {
            auto_return: 1,
            reserved: 0,
            duration: 0x123456789,
        };

        let mut writer = BitWriter::new();
        break_duration.encode(&mut writer).unwrap();
        let buffer = writer.finish();

        // Break duration should be 5 bytes (40 bits)
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.len(), break_duration.encoded_size());
    }

    #[test]
    fn test_splice_insert_encoding() {
        let splice_insert = SpliceInsert {
            splice_event_id: 1234,
            splice_event_cancel_indicator: 0,
            reserved: 0,
            out_of_network_indicator: 1,
            program_splice_flag: 1,
            duration_flag: 0,
            splice_immediate_flag: 1,
            reserved2: 0,
            splice_time: None,
            component_count: 0,
            components: Vec::new(),
            break_duration: None,
            unique_program_id: 5678,
            avail_num: 1,
            avails_expected: 1,
        };

        let mut writer = BitWriter::new();
        splice_insert.encode(&mut writer).unwrap();
        let buffer = writer.finish();

        // Verify the encoded size
        assert_eq!(buffer.len(), splice_insert.encoded_size());

        // Check that the splice_event_id is encoded correctly (first 4 bytes)
        let event_id = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        assert_eq!(event_id, 1234);
    }

    #[test]
    fn test_round_trip_with_builder() {
        // Create a message using builders
        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(
                SpliceInsertBuilder::new(1234)
                    .immediate()
                    .out_of_network(true)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // Encode to binary
        let encoded = section.encode_to_vec().unwrap();

        // Should have some reasonable size
        assert!(encoded.len() > 10);

        // First byte should be table_id (0xFC)
        assert_eq!(encoded[0], 0xFC);
    }

    #[cfg(feature = "crc-validation")]
    #[test]
    fn test_encode_with_crc() {
        use crate::encoding::CrcEncodable;

        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(SpliceInsertBuilder::new(5678).immediate().build().unwrap())
            .build()
            .unwrap();

        // Encode with CRC
        let encoded = section.encode_with_crc().unwrap();

        // Should have some reasonable size
        assert!(encoded.len() > 10);

        // Validate that the CRC is correct by parsing it back
        let parsed = crate::parser::parse_splice_info_section(&encoded).unwrap();
        assert!(parsed.validate_crc(&encoded).unwrap());
    }

    #[cfg(feature = "base64")]
    #[test]
    fn test_encode_base64() {
        use crate::encoding::Base64Encodable;

        let section = SpliceInfoSectionBuilder::new()
            .time_signal(TimeSignalBuilder::new().immediate().build().unwrap())
            .build()
            .unwrap();

        // Encode to base64
        let base64_string = section.encode_base64().unwrap();

        // Should be a valid base64 string
        assert!(!base64_string.is_empty());

        // Should be decodable
        use data_encoding::BASE64;
        let decoded = BASE64.decode(base64_string.as_bytes()).unwrap();
        assert!(decoded.len() > 10);
    }

    #[test]
    fn test_encoding_size_calculation() {
        let section = SpliceInfoSectionBuilder::new()
            .splice_insert(
                SpliceInsertBuilder::new(9999)
                    .immediate()
                    .out_of_network(true)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // The calculated size should match the actual encoded size
        let calculated_size = section.encoded_size();
        let encoded = section.encode_to_vec().unwrap();
        assert_eq!(calculated_size, encoded.len());
    }
}
