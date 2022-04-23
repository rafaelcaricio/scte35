
struct SegmentationDescriptor {
    tag: u8,
    descriptor_length: u8,
    identifier: u32,
    segmentation_event_id: u32,
    segmentation_event_cancel_indicator: bool,
    program_segmentation: Option<Vec<Component>>,
    delivery_restricted: Option<DeliveryRestriction>,
    segmentation_duration: Option<u64>,
    segmentation_upid: SegmentationUpid,
}

struct DeliveryRestriction {
    web_delivery_allowed_flag: bool,
    no_regional_blackout_flag: bool,
    archive_allowed_flag: bool,
    device_restrictions: DeviceRestrictions,
}

enum DeviceRestrictions {
    RestrictGroup0 = 0x00,
    RestrictGroup1 = 0x01,
    RestrictGroup2 = 0x10,
    None = 0x11,
}

enum SegmentationUpidType {
    NotUsed,
    UserDefinedDeprecated,
    ISCI,
    AdID,
    UMID,
    ISANDeprecated,
    ISAN,
    TID,
    TI,
    ADI,
    EIDR,
    ATSCContentIdentifier,
    MPU,
    MID,
    ADSInformation,
    URI,
    UUID,
    SCR,
    Reserved
}

enum SegmentationUpid {
    NotUsed,
    UserDefinedDeprecated,
    ISCI,
    AdID,
    UMID,
    ISANDeprecated,
    ISAN,
    TID,
    TI,
    ADI,
    EIDR,
    ATSCContentIdentifier,
    MPU,
    MID,
    ADSInformation,
    URI,
    UUID,
    SCR,
    Reserved
}

struct Component {
    component_tag: u8,
    pts_offset: u64,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
