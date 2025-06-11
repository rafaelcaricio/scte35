//! Builders for SCTE-35 descriptors.

use super::error::{BuilderError, BuilderResult, DurationExt};
use crate::descriptors::SegmentationDescriptor;
use crate::fmt::{format_identifier_to_string, format_private_data};
use crate::types::SegmentationType;
use crate::upid::SegmentationUpidType;
use std::time::Duration;

/// Builder for creating segmentation descriptors.
///
/// Segmentation descriptors provide detailed information about content segments
/// including timing and UPID data.
#[derive(Debug)]
pub struct SegmentationDescriptorBuilder {
    segmentation_event_id: Option<u32>,
    program_segmentation: bool,
    duration: Option<Duration>,
    delivery_restrictions: Option<DeliveryRestrictions>,
    upid: Option<Upid>,
    segmentation_type: SegmentationType,
    segment_num: u8,
    segments_expected: u8,
    sub_segmentation: Option<SubSegmentation>,
}

/// Delivery restrictions for segmentation descriptors.
#[derive(Debug, Clone)]
pub struct DeliveryRestrictions {
    /// Whether web delivery is allowed.
    pub web_delivery_allowed: bool,
    /// Whether regional blackout restrictions apply.
    pub no_regional_blackout: bool,
    /// Whether archival is allowed.
    pub archive_allowed: bool,
    /// Device-specific restrictions.
    pub device_restrictions: DeviceRestrictions,
}

/// Device restriction types.
#[derive(Debug, Clone, Copy)]
pub enum DeviceRestrictions {
    /// No device restrictions.
    None,
    /// Restrict group 1 devices.
    RestrictGroup1,
    /// Restrict group 2 devices.
    RestrictGroup2,
    /// Restrict both groups.
    RestrictBoth,
}

/// Sub-segmentation information.
#[derive(Debug, Clone)]
pub struct SubSegmentation {
    /// Sub-segment number.
    pub sub_segment_num: u8,
    /// Expected number of sub-segments.
    pub sub_segments_expected: u8,
}

/// UPID (Unique Program Identifier) types for segmentation descriptors.
#[derive(Debug, Clone)]
pub enum Upid {
    /// No UPID specified.
    None,
    /// User-defined UPID (deprecated).
    UserDefinedDeprecated(Vec<u8>),
    /// ISCI (Industry Standard Commercial Identifier).
    Isci(String),
    /// 12-character ASCII Ad ID.
    AdId(String),
    /// 32-byte UMID.
    Umid([u8; 32]),
    /// ISAN (International Standard Audiovisual Number) - deprecated.
    IsanDeprecated([u8; 12]),
    /// 12-byte ISAN.
    Isan([u8; 12]),
    /// 12-character ASCII TID.
    Tid(String),
    /// 8-byte Airing ID.
    AiringId(u64),
    /// ADI (Advertising Digital Identification).
    Adi(Vec<u8>),
    /// 12-byte EIDR.
    Eidr([u8; 12]),
    /// ATSC Content Identifier.
    AtscContentIdentifier(Vec<u8>),
    /// MPU (Media Processing Unit) with format identifier and private data.
    Mpu {
        /// 32-bit format identifier registered with SMPTE
        format_identifier: u32,
        /// Variable-length private data as defined by format identifier owner
        private_data: Vec<u8>,
    },
    /// MID (Media Identifier).
    Mid(Vec<u8>),
    /// ADS Information.
    AdsInformation(Vec<u8>),
    /// Variable-length URI.
    Uri(String),
    /// 16-byte UUID.
    Uuid([u8; 16]),
    /// SCR (Subscriber Company Reporting).
    Scr(Vec<u8>),
    /// Reserved or custom UPID type.
    Reserved(u8, Vec<u8>),
}

impl Upid {
    /// Creates a new MPU UPID with format identifier and private data.
    ///
    /// # Arguments
    /// * `format_identifier` - 32-bit SMPTE registered format identifier
    /// * `private_data` - Variable-length data as defined by format identifier owner
    ///
    /// # Example
    /// ```rust
    /// use scte35::builders::Upid;
    ///
    /// // Create MPU with custom format identifier and data
    /// let mpu = Upid::new_mpu(0x43554549, b"custom_content_id".to_vec());
    /// ```
    pub fn new_mpu(format_identifier: u32, private_data: Vec<u8>) -> Self {
        Upid::Mpu {
            format_identifier,
            private_data,
        }
    }

    /// Creates a new MPU UPID with format identifier and string data.
    ///
    /// This is a convenience method for text-based private data.
    ///
    /// # Example  
    /// ```rust
    /// use scte35::builders::Upid;
    ///
    /// // Create MPU with string content
    /// let mpu = Upid::new_mpu_str(0x43554549, "program_12345");
    /// ```
    pub fn new_mpu_str(format_identifier: u32, data: &str) -> Self {
        Upid::Mpu {
            format_identifier,
            private_data: data.as_bytes().to_vec(),
        }
    }
}

impl SegmentationDescriptorBuilder {
    /// Create a new segmentation descriptor builder.
    pub fn new(event_id: u32, segmentation_type: SegmentationType) -> Self {
        Self {
            segmentation_event_id: Some(event_id),
            program_segmentation: true,
            duration: None,
            delivery_restrictions: None,
            upid: None,
            segmentation_type,
            segment_num: 1,
            segments_expected: 1,
            sub_segmentation: None,
        }
    }

    /// Mark this segmentation event as cancelled.
    pub fn cancel_event(mut self) -> Self {
        self.segmentation_event_id = None;
        self
    }

    /// Set the duration of the segment.
    pub fn duration(mut self, duration: Duration) -> BuilderResult<Self> {
        let ticks = duration.to_pts_ticks();
        if ticks > 0x1_FFFF_FFFF {
            return Err(BuilderError::DurationTooLarge {
                field: "segmentation_duration",
                duration,
            });
        }
        self.duration = Some(duration);
        Ok(self)
    }

    /// Set no delivery restrictions.
    pub fn no_restrictions(mut self) -> Self {
        self.delivery_restrictions = None;
        self
    }

    /// Set delivery restrictions.
    pub fn delivery_restrictions(mut self, restrictions: DeliveryRestrictions) -> Self {
        self.delivery_restrictions = Some(restrictions);
        self
    }

    /// Set the UPID for this segment.
    pub fn upid(mut self, upid: Upid) -> BuilderResult<Self> {
        // Validate UPID based on type
        match &upid {
            Upid::Isci(s) | Upid::AdId(s) | Upid::Tid(s) => {
                if s.len() != 12 {
                    return Err(BuilderError::InvalidUpidLength {
                        expected: 12,
                        actual: s.len(),
                    });
                }
            }
            Upid::Mpu {
                format_identifier: _,
                private_data,
            } => {
                if private_data.len() > 251 {
                    return Err(BuilderError::InvalidValue {
                        field: "mpu_private_data",
                        reason: format!(
                            "MPU private data must be <= 251 bytes (4 bytes reserved for format_identifier). Got {} bytes",
                            private_data.len()
                        ),
                    });
                }
            }
            Upid::Uri(s) => {
                if s.is_empty() || s.len() > 255 {
                    return Err(BuilderError::InvalidValue {
                        field: "uri",
                        reason: "URI must be 1-255 bytes".to_string(),
                    });
                }
            }
            Upid::UserDefinedDeprecated(data)
            | Upid::Adi(data)
            | Upid::AtscContentIdentifier(data)
            | Upid::Mid(data)
            | Upid::AdsInformation(data)
            | Upid::Scr(data) => {
                if data.len() > 255 {
                    return Err(BuilderError::InvalidValue {
                        field: "upid_data",
                        reason: "UPID data must be <= 255 bytes".to_string(),
                    });
                }
            }
            Upid::Reserved(_, data) => {
                if data.len() > 255 {
                    return Err(BuilderError::InvalidValue {
                        field: "reserved_upid_data",
                        reason: "Reserved UPID data must be <= 255 bytes".to_string(),
                    });
                }
            }
            _ => {} // Other types have fixed sizes
        }
        self.upid = Some(upid);
        Ok(self)
    }

    /// Set segment numbering information.
    pub fn segment(mut self, num: u8, expected: u8) -> Self {
        self.segment_num = num;
        self.segments_expected = expected;
        self
    }

    /// Set sub-segment information.
    pub fn sub_segment(mut self, num: u8, expected: u8) -> Self {
        self.sub_segmentation = Some(SubSegmentation {
            sub_segment_num: num,
            sub_segments_expected: expected,
        });
        self
    }

    /// Build the segmentation descriptor.
    pub fn build(self) -> BuilderResult<SegmentationDescriptor> {
        let (event_id, cancel) = match self.segmentation_event_id {
            Some(id) => (id, false),
            None => (0, true),
        };

        let (delivery_not_restricted, web, blackout, archive, device) =
            match self.delivery_restrictions {
                None => (true, None, None, None, None),
                Some(r) => (
                    false,
                    Some(r.web_delivery_allowed),
                    Some(r.no_regional_blackout),
                    Some(r.archive_allowed),
                    Some(r.device_restrictions.into()),
                ),
            };

        let (upid_type, upid_bytes) = self.upid.unwrap_or(Upid::None).into();

        let duration_ticks = match self.duration {
            Some(duration) => {
                let ticks = duration.to_pts_ticks();
                if ticks > 0x1_FFFF_FFFF {
                    return Err(BuilderError::DurationTooLarge {
                        field: "segmentation_duration",
                        duration,
                    });
                }
                Some(ticks)
            }
            None => None,
        };

        Ok(SegmentationDescriptor {
            segmentation_event_id: event_id,
            segmentation_event_cancel_indicator: cancel,
            program_segmentation_flag: self.program_segmentation,
            segmentation_duration_flag: self.duration.is_some(),
            delivery_not_restricted_flag: delivery_not_restricted,
            web_delivery_allowed_flag: web,
            no_regional_blackout_flag: blackout,
            archive_allowed_flag: archive,
            device_restrictions: device,
            segmentation_duration: duration_ticks,
            segmentation_upid_type: upid_type,
            segmentation_upid_length: upid_bytes.len() as u8,
            segmentation_upid: upid_bytes,
            segmentation_type_id: self.segmentation_type.id(),
            segmentation_type: self.segmentation_type,
            segment_num: self.segment_num,
            segments_expected: self.segments_expected,
            sub_segment_num: self.sub_segmentation.as_ref().map(|s| s.sub_segment_num),
            sub_segments_expected: self
                .sub_segmentation
                .as_ref()
                .map(|s| s.sub_segments_expected),
        })
    }
}

impl From<DeviceRestrictions> for u8 {
    fn from(restrictions: DeviceRestrictions) -> Self {
        match restrictions {
            DeviceRestrictions::None => 0x00,
            DeviceRestrictions::RestrictGroup1 => 0x01,
            DeviceRestrictions::RestrictGroup2 => 0x02,
            DeviceRestrictions::RestrictBoth => 0x03,
        }
    }
}

impl From<Upid> for (SegmentationUpidType, Vec<u8>) {
    fn from(upid: Upid) -> Self {
        match upid {
            Upid::None => (SegmentationUpidType::NotUsed, vec![]),
            Upid::UserDefinedDeprecated(data) => {
                (SegmentationUpidType::UserDefinedDeprecated, data)
            }
            Upid::Isci(s) => (SegmentationUpidType::ISCI, s.into_bytes()),
            Upid::AdId(s) => (SegmentationUpidType::AdID, s.into_bytes()),
            Upid::Umid(bytes) => (SegmentationUpidType::UMID, bytes.to_vec()),
            Upid::IsanDeprecated(bytes) => (SegmentationUpidType::ISANDeprecated, bytes.to_vec()),
            Upid::Isan(bytes) => (SegmentationUpidType::ISAN, bytes.to_vec()),
            Upid::Tid(s) => (SegmentationUpidType::TID, s.into_bytes()),
            Upid::AiringId(id) => (SegmentationUpidType::AiringID, id.to_be_bytes().to_vec()),
            Upid::Adi(data) => (SegmentationUpidType::ADI, data),
            Upid::Eidr(bytes) => (SegmentationUpidType::EIDR, bytes.to_vec()),
            Upid::AtscContentIdentifier(data) => {
                (SegmentationUpidType::ATSCContentIdentifier, data)
            }
            Upid::Mpu {
                format_identifier,
                private_data,
            } => {
                let mut bytes = format_identifier.to_be_bytes().to_vec();
                bytes.extend(private_data);
                (SegmentationUpidType::MPU, bytes)
            }
            Upid::Mid(data) => (SegmentationUpidType::MID, data),
            Upid::AdsInformation(data) => (SegmentationUpidType::ADSInformation, data),
            Upid::Uri(s) => (SegmentationUpidType::URI, s.into_bytes()),
            Upid::Uuid(bytes) => (SegmentationUpidType::UUID, bytes.to_vec()),
            Upid::Scr(data) => (SegmentationUpidType::SCR, data),
            Upid::Reserved(type_id, data) => (SegmentationUpidType::Reserved(type_id), data),
        }
    }
}

/// Conversion from SegmentationDescriptor to Upid enum.
///
/// This enables round-trip functionality: parse -> convert to builder type -> modify -> rebuild.
/// Each UPID type has specific validation requirements according to SCTE-35 specification.
impl TryFrom<(&crate::descriptors::SegmentationDescriptor,)> for Upid {
    type Error = BuilderError;

    fn try_from(
        (descriptor,): (&crate::descriptors::SegmentationDescriptor,),
    ) -> Result<Self, Self::Error> {
        use crate::upid::SegmentationUpidType;

        let upid_bytes = &descriptor.segmentation_upid;

        match descriptor.segmentation_upid_type {
            SegmentationUpidType::NotUsed => Ok(Upid::None),
            SegmentationUpidType::UserDefinedDeprecated => {
                Ok(Upid::UserDefinedDeprecated(upid_bytes.clone()))
            }
            SegmentationUpidType::ISCI => {
                let s =
                    std::str::from_utf8(upid_bytes).map_err(|_| BuilderError::InvalidValue {
                        field: "isci_upid",
                        reason: "ISCI UPID must be valid UTF-8".to_string(),
                    })?;
                Ok(Upid::Isci(s.to_string()))
            }
            SegmentationUpidType::AdID => {
                let s =
                    std::str::from_utf8(upid_bytes).map_err(|_| BuilderError::InvalidValue {
                        field: "ad_id_upid",
                        reason: "Ad ID UPID must be valid UTF-8".to_string(),
                    })?;
                Ok(Upid::AdId(s.to_string()))
            }
            SegmentationUpidType::UMID => {
                if upid_bytes.len() != 32 {
                    return Err(BuilderError::InvalidValue {
                        field: "umid_upid",
                        reason: "UMID UPID must be exactly 32 bytes".to_string(),
                    });
                }
                let mut umid_array = [0u8; 32];
                umid_array.copy_from_slice(upid_bytes);
                Ok(Upid::Umid(umid_array))
            }
            SegmentationUpidType::ISANDeprecated => {
                if upid_bytes.len() != 12 {
                    return Err(BuilderError::InvalidValue {
                        field: "isan_deprecated_upid",
                        reason: "ISAN (deprecated) UPID must be exactly 12 bytes".to_string(),
                    });
                }
                let mut isan_array = [0u8; 12];
                isan_array.copy_from_slice(upid_bytes);
                Ok(Upid::IsanDeprecated(isan_array))
            }
            SegmentationUpidType::ISAN => {
                if upid_bytes.len() != 12 {
                    return Err(BuilderError::InvalidValue {
                        field: "isan_upid",
                        reason: "ISAN UPID must be exactly 12 bytes".to_string(),
                    });
                }
                let mut isan_array = [0u8; 12];
                isan_array.copy_from_slice(upid_bytes);
                Ok(Upid::Isan(isan_array))
            }
            SegmentationUpidType::TID => {
                let s =
                    std::str::from_utf8(upid_bytes).map_err(|_| BuilderError::InvalidValue {
                        field: "tid_upid",
                        reason: "TID UPID must be valid UTF-8".to_string(),
                    })?;
                Ok(Upid::Tid(s.to_string()))
            }
            SegmentationUpidType::AiringID => {
                if upid_bytes.len() != 8 {
                    return Err(BuilderError::InvalidValue {
                        field: "airing_id_upid",
                        reason: "Airing ID UPID must be exactly 8 bytes".to_string(),
                    });
                }
                let airing_id = u64::from_be_bytes([
                    upid_bytes[0],
                    upid_bytes[1],
                    upid_bytes[2],
                    upid_bytes[3],
                    upid_bytes[4],
                    upid_bytes[5],
                    upid_bytes[6],
                    upid_bytes[7],
                ]);
                Ok(Upid::AiringId(airing_id))
            }
            SegmentationUpidType::ADI => Ok(Upid::Adi(upid_bytes.clone())),
            SegmentationUpidType::EIDR => {
                if upid_bytes.len() != 12 {
                    return Err(BuilderError::InvalidValue {
                        field: "eidr_upid",
                        reason: "EIDR UPID must be exactly 12 bytes".to_string(),
                    });
                }
                let mut eidr_array = [0u8; 12];
                eidr_array.copy_from_slice(upid_bytes);
                Ok(Upid::Eidr(eidr_array))
            }
            SegmentationUpidType::ATSCContentIdentifier => {
                Ok(Upid::AtscContentIdentifier(upid_bytes.clone()))
            }
            SegmentationUpidType::MPU => {
                if upid_bytes.len() < 4 {
                    return Err(BuilderError::InvalidValue {
                        field: "mpu_upid",
                        reason: "MPU UPID must have at least 4 bytes for format_identifier"
                            .to_string(),
                    });
                }
                let format_identifier = u32::from_be_bytes([
                    upid_bytes[0],
                    upid_bytes[1],
                    upid_bytes[2],
                    upid_bytes[3],
                ]);
                let private_data = upid_bytes[4..].to_vec();
                Ok(Upid::Mpu {
                    format_identifier,
                    private_data,
                })
            }
            SegmentationUpidType::MID => Ok(Upid::Mid(upid_bytes.clone())),
            SegmentationUpidType::ADSInformation => Ok(Upid::AdsInformation(upid_bytes.clone())),
            SegmentationUpidType::URI => {
                let s =
                    std::str::from_utf8(upid_bytes).map_err(|_| BuilderError::InvalidValue {
                        field: "uri_upid",
                        reason: "URI UPID must be valid UTF-8".to_string(),
                    })?;
                Ok(Upid::Uri(s.to_string()))
            }
            SegmentationUpidType::UUID => {
                if upid_bytes.len() != 16 {
                    return Err(BuilderError::InvalidValue {
                        field: "uuid_upid",
                        reason: "UUID UPID must be exactly 16 bytes".to_string(),
                    });
                }
                let mut uuid_array = [0u8; 16];
                uuid_array.copy_from_slice(upid_bytes);
                Ok(Upid::Uuid(uuid_array))
            }
            SegmentationUpidType::SCR => Ok(Upid::Scr(upid_bytes.clone())),
            SegmentationUpidType::Reserved(type_id) => {
                Ok(Upid::Reserved(type_id, upid_bytes.clone()))
            }
        }
    }
}

impl std::fmt::Display for Upid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Upid::None => write!(f, "None"),
            Upid::UserDefinedDeprecated(data) => {
                write!(f, "UserDefinedDeprecated({} bytes)", data.len())
            }
            Upid::Isci(s) => write!(f, "ISCI(\"{s}\")"),
            Upid::AdId(s) => write!(f, "AdID(\"{s}\")"),
            Upid::Umid(bytes) => write!(f, "UMID({} bytes)", bytes.len()),
            Upid::IsanDeprecated(bytes) => write!(f, "ISANDeprecated({} bytes)", bytes.len()),
            Upid::Isan(bytes) => write!(f, "ISAN({} bytes)", bytes.len()),
            Upid::Tid(s) => write!(f, "TID(\"{s}\")"),
            Upid::AiringId(id) => write!(f, "AiringID({id})"),
            Upid::Adi(data) => write!(f, "ADI({} bytes)", data.len()),
            Upid::Eidr(bytes) => write!(f, "EIDR({} bytes)", bytes.len()),
            Upid::AtscContentIdentifier(data) => {
                write!(f, "ATSCContentIdentifier({} bytes)", data.len())
            }
            Upid::Mpu {
                format_identifier,
                private_data,
            } => {
                let format_str = format_identifier_to_string(*format_identifier);
                let data_str = format_private_data(private_data);
                write!(f, "MPU(format: {format_str}, data: {data_str})")
            }
            Upid::Mid(data) => write!(f, "MID({} bytes)", data.len()),
            Upid::AdsInformation(data) => write!(f, "ADSInformation({} bytes)", data.len()),
            Upid::Uri(s) => write!(f, "URI(\"{s}\")"),
            Upid::Uuid(bytes) => write!(f, "UUID({} bytes)", bytes.len()),
            Upid::Scr(data) => write!(f, "SCR({} bytes)", data.len()),
            Upid::Reserved(type_id, data) => {
                write!(f, "Reserved({}, {} bytes)", type_id, data.len())
            }
        }
    }
}
