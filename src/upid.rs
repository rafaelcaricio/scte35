//! UPID (Unique Program Identifier) types and formatting utilities.
//!
//! This module contains types and functions related to UPIDs used in
//! segmentation descriptors for content identification.

/// Represents the different types of UPIDs (Unique Program Identifiers) used in segmentation descriptors.
///
/// UPIDs provide standardized ways to identify content segments for various purposes
/// including ad insertion, content identification, and distribution control.
///
/// Each UPID type corresponds to a specific identifier format as defined in the SCTE-35 standard.
/// The numeric values represent the `segmentation_upid_type` field in segmentation descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SegmentationUpidType {
    /// No UPID is used (0x00)
    NotUsed,
    /// User-defined UPID (deprecated) (0x01)
    UserDefinedDeprecated,
    /// ISCI (Industry Standard Commercial Identifier) (0x02)
    ISCI,
    /// Ad Identifier (0x03)
    AdID,
    /// UMID (Unique Material Identifier) (0x04)
    UMID,
    /// ISAN (International Standard Audiovisual Number) - deprecated (0x05)
    ISANDeprecated,
    /// ISAN (International Standard Audiovisual Number) (0x06)
    ISAN,
    /// TID (Turner Identifier) (0x07)
    TID,
    /// AiringID (0x08)
    AiringID,
    /// ADI (Advertising Digital Identification) (0x09)
    ADI,
    /// EIDR (Entertainment Identifier Registry) (0x0A)
    EIDR,
    /// ATSC Content Identifier (0x0B)
    ATSCContentIdentifier,
    /// MPU (Media Processing Unit) (0x0C)
    MPU,
    /// MID (Media Identifier) (0x0D)
    MID,
    /// ADS Information (0x0E)
    ADSInformation,
    /// URI (Uniform Resource Identifier) (0x0F)
    URI,
    /// UUID (Universally Unique Identifier) (0x10)
    UUID,
    /// SCR (Subscriber Company Reporting) (0x11)
    SCR,
    /// Reserved or unknown UPID type
    Reserved(u8),
}

impl Default for SegmentationUpidType {
    fn default() -> Self {
        SegmentationUpidType::NotUsed
    }
}

impl From<SegmentationUpidType> for u8 {
    fn from(s: SegmentationUpidType) -> Self {
        use SegmentationUpidType::*;
        match s {
            NotUsed => 0x00,
            UserDefinedDeprecated => 0x01,
            ISCI => 0x02,
            AdID => 0x03,
            UMID => 0x04,
            ISANDeprecated => 0x05,
            ISAN => 0x06,
            TID => 0x07,
            AiringID => 0x08,
            ADI => 0x09,
            EIDR => 0x0A,
            ATSCContentIdentifier => 0x0B,
            MPU => 0x0C,
            MID => 0x0D,
            ADSInformation => 0x0E,
            URI => 0x0F,
            UUID => 0x10,
            SCR => 0x11,
            Reserved(x) => x,
        }
    }
}

impl From<u8> for SegmentationUpidType {
    fn from(value: u8) -> Self {
        use SegmentationUpidType::*;
        match value {
            0x00 => NotUsed,
            0x01 => UserDefinedDeprecated,
            0x02 => ISCI,
            0x03 => AdID,
            0x04 => UMID,
            0x05 => ISANDeprecated,
            0x06 => ISAN,
            0x07 => TID,
            0x08 => AiringID,
            0x09 => ADI,
            0x0A => EIDR,
            0x0B => ATSCContentIdentifier,
            0x0C => MPU,
            0x0D => MID,
            0x0E => ADSInformation,
            0x0F => URI,
            0x10 => UUID,
            0x11 => SCR,
            x => Reserved(x),
        }
    }
}

impl SegmentationUpidType {
    /// Returns a human-readable description of the UPID type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use scte35_parsing::SegmentationUpidType;
    ///
    /// let upid_type = SegmentationUpidType::AdID;
    /// assert_eq!(upid_type.description(), "Ad Identifier");
    /// ```
    pub fn description(&self) -> &'static str {
        use SegmentationUpidType::*;
        match self {
            NotUsed => "Not Used",
            UserDefinedDeprecated => "User Defined (Deprecated)",
            ISCI => "ISCI (Industry Standard Commercial Identifier)",
            AdID => "Ad Identifier",
            UMID => "UMID (Unique Material Identifier)",
            ISANDeprecated => "ISAN (Deprecated)",
            ISAN => "ISAN (International Standard Audiovisual Number)",
            TID => "TID (Turner Identifier)",
            AiringID => "Airing ID",
            ADI => "ADI (Advertising Digital Identification)",
            EIDR => "EIDR (Entertainment Identifier Registry)",
            ATSCContentIdentifier => "ATSC Content Identifier",
            MPU => "MPU (Media Processing Unit)",
            MID => "MID (Media Identifier)",
            ADSInformation => "ADS Information",
            URI => "URI (Uniform Resource Identifier)",
            UUID => "UUID (Universally Unique Identifier)",
            SCR => "SCR (Subscriber Company Reporting)",
            Reserved(_) => "Reserved/Unknown",
        }
    }
}

/// Helper function to format UUID bytes as a standard UUID string.
pub fn format_uuid(bytes: &[u8]) -> String {
    if bytes.len() != 16 {
        return format_base64(bytes);
    }
    
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11],
        bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

/// Helper function to format ISAN bytes as an ISAN string.
pub fn format_isan(bytes: &[u8]) -> String {
    if bytes.len() >= 12 {
        // ISAN format: XXXX-XXXX-XXXX-XXXX-XXXX-X (using hex representation)
        format!(
            "{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11]
        )
    } else {
        format_base64(bytes)
    }
}

/// Helper function to format bytes as base64 string, with fallback when base64 feature is disabled.
#[cfg(any(feature = "base64", test))]
pub fn format_base64(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose, Engine};
    general_purpose::STANDARD.encode(bytes)
}

/// Fallback when base64 feature is disabled - returns empty string.
#[cfg(not(any(feature = "base64", test)))]
pub fn format_base64(_bytes: &[u8]) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upid_type_conversion() {
        assert_eq!(u8::from(SegmentationUpidType::NotUsed), 0x00);
        assert_eq!(u8::from(SegmentationUpidType::AdID), 0x03);
        assert_eq!(u8::from(SegmentationUpidType::UUID), 0x10);
        assert_eq!(u8::from(SegmentationUpidType::Reserved(0xFF)), 0xFF);
    }

    #[test]
    fn test_upid_type_from_u8() {
        assert_eq!(SegmentationUpidType::from(0x00), SegmentationUpidType::NotUsed);
        assert_eq!(SegmentationUpidType::from(0x03), SegmentationUpidType::AdID);
        assert_eq!(SegmentationUpidType::from(0x10), SegmentationUpidType::UUID);
        assert_eq!(SegmentationUpidType::from(0xFF), SegmentationUpidType::Reserved(0xFF));
    }

    #[test]
    fn test_upid_type_description() {
        assert_eq!(SegmentationUpidType::AdID.description(), "Ad Identifier");
        assert_eq!(SegmentationUpidType::UUID.description(), "UUID (Universally Unique Identifier)");
    }

    #[test]
    fn test_format_uuid() {
        let uuid_bytes = vec![
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0
        ];
        let formatted = format_uuid(&uuid_bytes);
        assert_eq!(formatted, "12345678-9abc-def0-1234-56789abcdef0");
    }

    #[test]
    fn test_format_isan() {
        let isan_bytes = vec![
            0x00, 0x00, 0x00, 0x01, 0x23, 0x45,
            0x67, 0x89, 0xab, 0xcd, 0xef, 0x00
        ];
        let formatted = format_isan(&isan_bytes);
        assert_eq!(formatted, "0000-0001-2345-6789-abcd-ef00");
    }
}