//! Builders for SCTE-35 descriptors.

use crate::descriptors::SegmentationDescriptor;
use crate::types::SegmentationType;
use crate::upid::SegmentationUpidType;
use super::error::{BuilderError, BuilderResult, DurationExt};
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
    /// MPU (Media Processing Unit).
    Mpu(Vec<u8>),
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
            return Err(BuilderError::DurationTooLarge { field: "segmentation_duration", duration });
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
                    return Err(BuilderError::InvalidUpidLength { expected: 12, actual: s.len() });
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
            Upid::UserDefinedDeprecated(data) | Upid::Adi(data) | Upid::AtscContentIdentifier(data) 
            | Upid::Mpu(data) | Upid::Mid(data) | Upid::AdsInformation(data) | Upid::Scr(data) => {
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
            _ => {}  // Other types have fixed sizes
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
                Some(r) => (false, 
                    Some(r.web_delivery_allowed),
                    Some(r.no_regional_blackout),
                    Some(r.archive_allowed),
                    Some(r.device_restrictions.into())),
            };

        let (upid_type, upid_bytes) = self.upid.unwrap_or(Upid::None).into();

        let duration_ticks = match self.duration {
            Some(duration) => {
                let ticks = duration.to_pts_ticks();
                if ticks > 0x1_FFFF_FFFF {
                    return Err(BuilderError::DurationTooLarge { field: "segmentation_duration", duration });
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
            sub_segments_expected: self.sub_segmentation.as_ref().map(|s| s.sub_segments_expected),
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
            Upid::UserDefinedDeprecated(data) => (SegmentationUpidType::UserDefinedDeprecated, data),
            Upid::Isci(s) => (SegmentationUpidType::ISCI, s.into_bytes()),
            Upid::AdId(s) => (SegmentationUpidType::AdID, s.into_bytes()),
            Upid::Umid(bytes) => (SegmentationUpidType::UMID, bytes.to_vec()),
            Upid::IsanDeprecated(bytes) => (SegmentationUpidType::ISANDeprecated, bytes.to_vec()),
            Upid::Isan(bytes) => (SegmentationUpidType::ISAN, bytes.to_vec()),
            Upid::Tid(s) => (SegmentationUpidType::TID, s.into_bytes()),
            Upid::AiringId(id) => (SegmentationUpidType::AiringID, id.to_be_bytes().to_vec()),
            Upid::Adi(data) => (SegmentationUpidType::ADI, data),
            Upid::Eidr(bytes) => (SegmentationUpidType::EIDR, bytes.to_vec()),
            Upid::AtscContentIdentifier(data) => (SegmentationUpidType::ATSCContentIdentifier, data),
            Upid::Mpu(data) => (SegmentationUpidType::MPU, data),
            Upid::Mid(data) => (SegmentationUpidType::MID, data),
            Upid::AdsInformation(data) => (SegmentationUpidType::ADSInformation, data),
            Upid::Uri(s) => (SegmentationUpidType::URI, s.into_bytes()),
            Upid::Uuid(bytes) => (SegmentationUpidType::UUID, bytes.to_vec()),
            Upid::Scr(data) => (SegmentationUpidType::SCR, data),
            Upid::Reserved(type_id, data) => (SegmentationUpidType::Reserved(type_id), data),
        }
    }
}