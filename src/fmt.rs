//! Formatting utilities for SCTE-35 data structures.
//!
//! This module provides common formatting functions for displaying
//! SCTE-35 data in human-readable formats, with intelligent handling
//! of binary vs text data.

/// Converts a 32-bit format identifier to a human-readable string.
///
/// Returns ASCII representation if all bytes are printable ASCII letters/numbers,
/// otherwise returns hex representation.
///
/// # Arguments
/// * `format_identifier` - 32-bit format identifier to format
///
/// # Examples
/// ```rust
/// use scte35::fmt::format_identifier_to_string;
///
/// assert_eq!(format_identifier_to_string(0x43554549), "CUEI");
/// assert_eq!(format_identifier_to_string(0x12345678), "0x12345678");
/// ```
pub fn format_identifier_to_string(format_identifier: u32) -> String {
    let bytes = format_identifier.to_be_bytes();

    // Check if all bytes are printable ASCII letters or numbers
    if bytes.iter().all(|&b| b.is_ascii_alphanumeric()) {
        // Convert to ASCII string
        String::from_utf8_lossy(&bytes).to_string()
    } else {
        // Fallback to hex representation
        format!("0x{:08X}", format_identifier)
    }
}

/// Formats private data for display, showing as string if valid UTF-8,
/// otherwise as hex with length limit for readability.
///
/// # Arguments
/// * `data` - Byte slice to format for display
///
/// # Examples
/// ```rust
/// use scte35::fmt::format_private_data;
///
/// assert_eq!(format_private_data(b"test"), "\"test\"");
/// assert_eq!(format_private_data(&[0x01, 0x02, 0x03]), "0x010203");
/// assert_eq!(format_private_data(&[]), "empty");
/// ```
pub fn format_private_data(data: &[u8]) -> String {
    if data.is_empty() {
        return "empty".to_string();
    }

    // Try to interpret as UTF-8 string first
    if let Ok(s) = std::str::from_utf8(data) {
        // Check if it's printable (no control characters except space)
        if s.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            // Truncate long strings for readability
            if s.len() <= 50 {
                format!("\"{}\"", s)
            } else {
                format!("\"{}...\" ({} bytes)", &s[..47], data.len())
            }
        } else {
            // Contains control characters, show as hex
            format_as_hex(data)
        }
    } else {
        // Not valid UTF-8, show as hex
        format_as_hex(data)
    }
}

/// Formats data as hex string with length limit for readability.
///
/// # Arguments
/// * `data` - Byte slice to format as hexadecimal
///
/// # Examples
/// ```rust
/// use scte35::fmt::format_as_hex;
///
/// assert_eq!(format_as_hex(&[0x01, 0x02, 0x03]), "0x010203");
/// assert_eq!(format_as_hex(&(0..20).collect::<Vec<u8>>()), "0x000102030405... (20 bytes)");
/// ```
pub fn format_as_hex(data: &[u8]) -> String {
    if data.len() <= 8 {
        // Show all bytes for short data
        format!(
            "0x{}",
            data.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        )
    } else {
        // Show first few bytes with truncation for long data
        let preview: String = data[..6].iter().map(|b| format!("{:02x}", b)).collect();
        format!("0x{}... ({} bytes)", preview, data.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_identifier_to_string() {
        // ASCII alphanumeric
        assert_eq!(format_identifier_to_string(0x43554549), "CUEI");
        assert_eq!(format_identifier_to_string(0x54455354), "TEST");
        assert_eq!(format_identifier_to_string(0x41424344), "ABCD");

        // Non-ASCII or non-alphanumeric
        assert_eq!(format_identifier_to_string(0x12345678), "0x12345678");
        assert_eq!(format_identifier_to_string(0x41422D44), "0x41422D44"); // "AB-D"
        assert_eq!(format_identifier_to_string(0x00000000), "0x00000000");
    }

    #[test]
    fn test_format_private_data() {
        // Empty data
        assert_eq!(format_private_data(&[]), "empty");

        // Valid UTF-8 string
        assert_eq!(format_private_data(b"test"), "\"test\"");
        assert_eq!(format_private_data(b"hello world"), "\"hello world\"");

        // Binary data
        assert_eq!(format_private_data(&[0x01, 0x02, 0x03]), "0x010203");
        assert_eq!(format_private_data(&[0xFF, 0xFE, 0xFD]), "0xfffefd");

        // Long string (truncated)
        let long_string = "a".repeat(60);
        let result = format_private_data(long_string.as_bytes());
        assert!(result.starts_with("\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...\""));
        assert!(result.contains("(60 bytes)"));

        // String with control characters
        assert_eq!(format_private_data(b"test\x00\x01"), "0x746573740001");
    }

    #[test]
    fn test_format_as_hex() {
        // Short data
        assert_eq!(format_as_hex(&[]), "0x");
        assert_eq!(format_as_hex(&[0x01]), "0x01");
        assert_eq!(format_as_hex(&[0x01, 0x02, 0x03, 0x04]), "0x01020304");
        assert_eq!(
            format_as_hex(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]),
            "0x0102030405060708"
        );

        // Long data (truncated)
        let long_data: Vec<u8> = (0..20).collect();
        assert_eq!(format_as_hex(&long_data), "0x000102030405... (20 bytes)");
    }
}
