//! Example demonstrating serde serialization of SCTE-35 messages

use data_encoding::BASE64;
use scte35::parse_splice_info_section;

fn main() {
    // Example SCTE-35 message with segmentation descriptor
    let base64_message = "/DAvAAAAAAAA///wBQb+dGKQoAAZAhdDVUVJSAAAjn+fCAgAAAAALKChijUCAKnMZ1g=";
    let buffer = BASE64.decode(base64_message.as_bytes()).unwrap();

    let section = parse_splice_info_section(&buffer).unwrap();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&section).unwrap();
    println!("JSON representation:");
    println!("{json}");

    // Demonstrate that we can deserialize back
    let deserialized: scte35::SpliceInfoSection = serde_json::from_str(&json).unwrap();

    // Verify key fields match
    assert_eq!(section.table_id, deserialized.table_id);
    assert_eq!(
        section.splice_command_type,
        deserialized.splice_command_type
    );
    assert_eq!(section.crc_32, deserialized.crc_32);

    println!("\nSuccessfully round-tripped through JSON!");
}
