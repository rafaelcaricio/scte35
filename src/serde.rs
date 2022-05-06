use crate::commands::SpliceCommand;
use crate::descriptors::{SegmentationDescriptor, SegmentationUpid, SpliceDescriptorExt};
use crate::info::EncodedData;
use crate::{SpliceDescriptor, SpliceInfoSection, SpliceTime};
use ascii::AsciiStr;
use bitstream_io::{BigEndian, BitRecorder, BitWriter};
use serde::ser::{Error, SerializeStruct};
use serde::{Serialize, Serializer};
use std::fmt;
use std::fmt::LowerHex;
use std::time::Duration;

/// Truncate to 6 decimal positions, as shown in the spec.
fn ticks_to_secs(value: u64) -> f64 {
    (value as f64 / 90_000.0 * 1_000_000.0).ceil() as f64 / 1_000_000.0
}

impl Serialize for SegmentationDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use crate::SegmentationFieldSyntax::*;

        #[inline]
        fn as_hex<T>(value: T) -> String
        where
            T: LowerHex,
        {
            format!("0x{:02x}", value)
        }

        let segmentation_syntax = self.segmentation_type().syntax();

        // predict number of fields in the struct
        let mut num_fields = 6;
        if !self.segmentation_event_cancel_indicator() {
            num_fields += 12;
            if !self.delivery_not_restricted_flag() {
                num_fields += 4;
            }
            if self.segmentation_duration_flag() {
                num_fields += 2;
            }
            match segmentation_syntax.sub_segment_num() {
                Fixed(_) | NonZero | ZeroOrNonZero => {
                    num_fields += 1;
                }
                NotUsed => {}
            }
            match segmentation_syntax.sub_segments_expected() {
                Fixed(_) | NonZero | ZeroOrNonZero => {
                    num_fields += 1;
                }
                NotUsed => {}
            }
        }

        let mut state = serializer.serialize_struct("SegmentationDescriptor", num_fields)?;
        state.serialize_field("name", "Segmentation Descriptor")?;
        state.serialize_field(
            "splice_descriptor_tag",
            &as_hex(self.splice_descriptor_tag()),
        )?;
        state.serialize_field("descriptor_length", &self.descriptor_length())?;
        let id = self.identifier().to_be_bytes();
        state.serialize_field(
            "identifier",
            AsciiStr::from_ascii(id.as_slice())
                .expect("ascii characters")
                .as_str(),
        )?;
        state.serialize_field(
            "segmentation_event_id",
            &as_hex(self.segmentation_event_id()),
        )?;
        state.serialize_field(
            "segmentation_event_cancel_indicator",
            &self.segmentation_event_cancel_indicator(),
        )?;

        if !self.segmentation_event_cancel_indicator() {
            state.serialize_field(
                "program_segmentation_flag",
                &self.program_segmentation_flag(),
            )?;
            state.serialize_field(
                "segmentation_duration_flag",
                &self.segmentation_duration_flag(),
            )?;
            state.serialize_field(
                "delivery_not_restricted_flag",
                &self.delivery_not_restricted_flag(),
            )?;
            if !self.delivery_not_restricted_flag() {
                state.serialize_field(
                    "web_delivery_allowed_flag",
                    &self.web_delivery_allowed_flag(),
                )?;
                state.serialize_field(
                    "no_regional_blackout_flag",
                    &self.no_regional_blackout_flag(),
                )?;
                state.serialize_field("archive_allowed_flag", &self.archive_allowed_flag())?;
                state.serialize_field("device_restrictions", &self.device_restrictions())?;
            }
            state.serialize_field("components", self.components())?;
            if self.segmentation_duration_flag() {
                let duration_secs = ticks_to_secs(self.segmentation_duration());
                state.serialize_field("segmentation_duration", &self.segmentation_duration())?;
                state.serialize_field("segmentation_duration_secs", &duration_secs)?;
                state.serialize_field(
                    "segmentation_duration_human",
                    &format_duration(Duration::from_secs_f64(duration_secs)).to_string(),
                )?;
            }
            state.serialize_field(
                "segmentation_upid_type",
                &as_hex(u8::from(self.segmentation_upid().segmentation_upid_type())),
            )?;
            state.serialize_field(
                "segmentation_upid_type_name",
                &format!("{}", self.segmentation_upid().segmentation_upid_type()),
            )?;
            state.serialize_field(
                "segmentation_upid_length",
                &self.segmentation_upid().segmentation_upid_length(),
            )?;
            state.serialize_field("segmentation_upid", &self.segmentation_upid())?;
            state.serialize_field(
                "segmentation_message",
                &format!("{}", self.segmentation_type()),
            )?;
            state.serialize_field("segmentation_type_id", &self.segmentation_type().id())?;
            state.serialize_field("segment_num", &self.segment_num())?;
            state.serialize_field("segments_expected", &self.segments_expected())?;
            match segmentation_syntax.sub_segment_num() {
                Fixed(v) => {
                    state.serialize_field("sub_segment_num", &v)?;
                }
                NonZero | ZeroOrNonZero => {
                    state.serialize_field("sub_segment_num", &self.sub_segment_num())?;
                }
                NotUsed => {}
            }
            match segmentation_syntax.sub_segments_expected() {
                Fixed(v) => {
                    state.serialize_field("sub_segments_expected", &v)?;
                }
                NonZero | ZeroOrNonZero => {
                    state
                        .serialize_field("sub_segments_expected", &self.sub_segments_expected())?;
                }
                NotUsed => {}
            }
        }
        state.end()
    }
}

impl Serialize for SegmentationUpid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use SegmentationUpid::*;

        let mut recorder = BitRecorder::<u32, BigEndian>::new();
        self.write_to(&mut recorder)
            .map_err(|err| S::Error::custom(format!("{}", err)))?;

        let mut data = Vec::new();
        let mut buffer = BitWriter::endian(&mut data, BigEndian);
        recorder
            .playback(&mut buffer)
            .map_err(|err| S::Error::custom(format!("{}", err)))?;

        // TODO: serialize as struct when variant is MPU and MID
        match self {
            // if field is represented as a character, then show with textual representation
            ISCI(v) | AdID(v) | TID(v) | ADSInformation(v) | URI(v) | SCR(v) => {
                serializer.serialize_str(v.as_str())
            }
            // if field is represented as a number, then show as hex
            ISAN(v) | EIDR(v) | UUID(v) => serializer.serialize_str(&format!("0x{:x}", v)),
            ISANDeprecated(v) | AiringID(v) => serializer.serialize_str(&format!("0x{:x}", v)),
            // everything else show as hex, we skip the first byte (which is the length)
            _ => serializer.serialize_str(&format!("0x{}", hex::encode(&data[1..]))),
        }
    }
}

impl Serialize for SpliceTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let num_fields = if self.time_specified_flag() { 3 } else { 1 };

        let mut state = serializer.serialize_struct("SpliceTime", num_fields)?;
        state.serialize_field("time_specified_flag", &self.time_specified_flag())?;
        if self.time_specified_flag() {
            state.serialize_field("pts_time", &self.pts_time().unwrap_or(0))?;
            state.serialize_field(
                "pts_time_secs",
                &ticks_to_secs(self.pts_time().unwrap_or(0)),
            )?;
        }
        state.end()
    }
}

impl Serialize for SpliceDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use SpliceDescriptor::*;
        match self {
            Segmentation(seg) => seg.serialize(serializer),
            Unknown(tag, len, data) => {
                let mut struc = serializer.serialize_struct("SpliceDescriptor", 3)?;
                struc.serialize_field("tag", &format!("0x{:x}", tag))?;
                struc.serialize_field("length", &len)?;
                struc.serialize_field("data", &format!("0x{}", hex::encode(data).as_str()))?;
                struc.end()
            }
            // TODO: add other descriptors
            _ => serializer.serialize_str(&format!("{:?}", self)),
        }
    }
}

impl<C> Serialize for SpliceInfoSection<C, EncodedData>
where
    C: SpliceCommand + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[inline]
        fn as_hex<T>(value: T) -> String
        where
            T: LowerHex,
        {
            format!("0x{:x}", value)
        }

        let mut state = serializer.serialize_struct("SpliceInfoSection", 19)?;
        state.serialize_field("table_id", &as_hex(self.state.table_id))?;
        state.serialize_field(
            "section_syntax_indicator",
            &self.state.section_syntax_indicator,
        )?;
        state.serialize_field("private_indicator", &self.state.private_indicator)?;
        state.serialize_field("sap_type", &as_hex(self.state.sap_type as u8))?;
        state.serialize_field("section_length", &self.encoded.section_length)?;
        state.serialize_field("protocol_version", &self.state.protocol_version)?;
        state.serialize_field("encrypted_packet", &self.state.encrypted_packet)?;
        state.serialize_field(
            "encryption_algorithm",
            &u8::from(self.state.encryption_algorithm),
        )?;
        state.serialize_field("pts_adjustment", &self.state.pts_adjustment)?;
        let pts_adjustment_secs = ticks_to_secs(self.state.pts_adjustment);
        state.serialize_field("pts_adjustment_secs", &pts_adjustment_secs)?;
        state.serialize_field(
            "pts_adjustment_human",
            &format_duration(Duration::from_secs_f64(pts_adjustment_secs)).to_string(),
        )?;
        state.serialize_field("cw_index", &as_hex(self.state.cw_index))?;
        state.serialize_field("tier", &as_hex(self.state.tier))?;
        state.serialize_field("splice_command_length", &self.encoded.splice_command_length)?;
        state.serialize_field(
            "splice_command_type",
            &u8::from(self.encoded.splice_command_type),
        )?;
        state.serialize_field("splice_command_name", &self.encoded.splice_command_type)?;
        state.serialize_field("splice_command", &self.state.splice_command)?;
        state.serialize_field(
            "descriptor_loop_length",
            &self.encoded.descriptor_loop_length,
        )?;
        state.serialize_field("descriptors", &self.state.descriptors)?;
        state.serialize_field("crc_32", &as_hex(self.encoded.crc32))?;
        state.end()
    }
}

// Copyright (c) 2016 The humantime Developers
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
/// Formats duration into a human-readable string
///
/// Note: this format is guaranteed to have same value when using
/// parse_duration, but we can change some details of the exact composition
/// of the value.
pub(crate) fn format_duration(val: Duration) -> FormattedDuration {
    FormattedDuration(val)
}

fn item_plural(f: &mut fmt::Formatter, started: &mut bool, name: &str, value: u64) -> fmt::Result {
    if value > 0 {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", value, name)?;
        if value > 1 {
            f.write_str("s")?;
        }
        *started = true;
    }
    Ok(())
}

fn item(f: &mut fmt::Formatter, started: &mut bool, name: &str, value: u32) -> fmt::Result {
    if value > 0 {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", value, name)?;
        *started = true;
    }
    Ok(())
}

/// A wrapper type that allows you to Display a Duration
#[derive(Debug, Clone)]
pub(crate) struct FormattedDuration(Duration);

impl fmt::Display for FormattedDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let secs = self.0.as_secs();
        let nanos = self.0.subsec_nanos();

        if secs == 0 && nanos == 0 {
            f.write_str("0s")?;
            return Ok(());
        }

        let years = secs / 31_557_600; // 365.25d
        let ydays = secs % 31_557_600;
        let months = ydays / 2_630_016; // 30.44d
        let mdays = ydays % 2_630_016;
        let days = mdays / 86400;
        let day_secs = mdays % 86400;
        let hours = day_secs / 3600;
        let minutes = day_secs % 3600 / 60;
        let seconds = day_secs % 60;

        let millis = nanos / 1_000_000;
        let micros = nanos / 1000 % 1000;
        let nanosec = nanos % 1000;

        let started = &mut false;
        item_plural(f, started, "year", years)?;
        item_plural(f, started, "month", months)?;
        item_plural(f, started, "day", days)?;
        item(f, started, "h", hours as u32)?;
        item(f, started, "m", minutes as u32)?;
        item(f, started, "s", seconds as u32)?;
        item(f, started, "milli", millis)?;
        item(f, started, "us", micros)?;
        item(f, started, "ns", nanosec)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::TimeSignal;
    use crate::descriptors::SegmentationType;
    use crate::{ClockTimeExt, SpliceNull};
    use anyhow::Result;
    use assert_json_diff::assert_json_eq;
    use std::time::Duration;

    #[test]
    fn test_ticks_to_secs() {
        let time = Duration::from_secs_f64(21388.766756);
        assert_eq!(time.to_90k(), 0x072bd0050);
        assert_eq!(ticks_to_secs(0x072bd0050), 21388.766756);
        assert_eq!(ticks_to_secs(time.to_90k()), 21388.766756);
    }

    #[test]
    fn serialize_splice_null() -> Result<()> {
        let splice_null = SpliceNull::default();
        assert_json_eq!(serde_json::to_value(&splice_null)?, serde_json::json!({}));
        Ok(())
    }

    #[test]
    fn serialize_time_signal_without_time() -> Result<()> {
        let time_signal = TimeSignal::default();
        assert_json_eq!(
            serde_json::to_value(&time_signal)?,
            serde_json::json!({
                "time_specified_flag": false
            })
        );
        Ok(())
    }

    #[test]
    fn serialize_time_signal_with_time() -> Result<()> {
        let time_signal = TimeSignal::from(Duration::from_secs(10));
        assert_json_eq!(
            serde_json::to_value(&time_signal)?,
            serde_json::json!({
                "time_specified_flag": true,
                "pts_time": 900000,
                "pts_time_secs": 10.0
            })
        );
        Ok(())
    }

    #[test]
    fn serialize_segmentation_to_json() -> Result<()> {
        let mut descriptor = SegmentationDescriptor::default();
        descriptor.set_segmentation_event_id(0x4800008e);
        descriptor.set_program_segmentation_flag(true);
        descriptor.set_segmentation_duration_flag(true);
        descriptor.set_no_regional_blackout_flag(true);
        descriptor.set_archive_allowed_flag(true);
        descriptor.set_segmentation_duration(Duration::from_secs_f32(307.0));
        descriptor.set_segmentation_duration(27630000);
        descriptor.set_segmentation_upid(SegmentationUpid::AiringID(0x2ca0a18a));
        descriptor.set_segmentation_type(SegmentationType::ProviderPlacementOpportunityStart);
        descriptor.set_segment_num(2);
        descriptor.set_sub_segment_num(154);
        descriptor.set_sub_segments_expected(201);

        // We need to write to calculate the length
        let mut data = Vec::new();
        descriptor.write_to(&mut data)?;

        let expected_json: serde_json::Value = serde_json::from_str(
            r#"{
            "name": "Segmentation Descriptor",
            "splice_descriptor_tag": "0x02",
            "descriptor_length": 30,
            "identifier": "CUEI",
            "segmentation_event_id": "0x4800008e",
            "segmentation_event_cancel_indicator": false,
            "program_segmentation_flag": true,
            "segmentation_duration_flag": true,
            "delivery_not_restricted_flag": false,
            "web_delivery_allowed_flag": false,
            "no_regional_blackout_flag": true,
            "archive_allowed_flag": true,
            "device_restrictions": "None",
            "components": [],
            "segmentation_duration": 27630000,
            "segmentation_duration_secs": 307.0,
            "segmentation_duration_human": "5m 7s",
            "segmentation_upid_type": "0x08",
            "segmentation_upid_type_name": "AiringID",
            "segmentation_upid_length": 8,
            "segmentation_upid": "0x2ca0a18a",
            "segmentation_message": "Provider Placement Opportunity Start",
            "segmentation_type_id": 52,
            "segment_num": 2,
            "segments_expected": 0,
            "sub_segment_num": 154,
            "sub_segments_expected": 201
        }"#,
        )?;

        assert_json_eq!(serde_json::to_value(&descriptor)?, expected_json);
        Ok(())
    }

    #[test]
    fn serialize_segmentation_with_cancel_indicator_to_json() -> Result<()> {
        let mut descriptor = SegmentationDescriptor::default();
        descriptor.set_segmentation_event_id(0x4800008e);
        descriptor.set_segmentation_event_cancel_indicator(true);

        // We need to write to calculate the length
        let mut data = Vec::new();
        descriptor.write_to(&mut data)?;

        let expected_json: serde_json::Value = serde_json::from_str(
            r#"{
            "name": "Segmentation Descriptor",
            "splice_descriptor_tag": "0x02",
            "descriptor_length": 9,
            "identifier": "CUEI",
            "segmentation_event_id": "0x4800008e",
            "segmentation_event_cancel_indicator": true
        }"#,
        )?;

        assert_json_eq!(serde_json::to_value(&descriptor)?, expected_json);
        Ok(())
    }

    #[test]
    fn compliance_spec_14_1_example_time_signal_as_json() -> Result<()> {
        let mut splice = SpliceInfoSection::new(TimeSignal::from(0x072bd0050u64));
        splice.set_cw_index(0xff);

        let mut descriptor = SegmentationDescriptor::default();
        descriptor.set_segmentation_event_id(0x4800008e);
        descriptor.set_program_segmentation_flag(true);
        descriptor.set_segmentation_duration_flag(true);
        descriptor.set_no_regional_blackout_flag(true);
        descriptor.set_archive_allowed_flag(true);
        descriptor.set_segmentation_duration(27630000);
        descriptor.set_segmentation_upid(SegmentationUpid::AiringID(0x2ca0a18a));
        descriptor.set_segmentation_type(SegmentationType::ProviderPlacementOpportunityStart);
        descriptor.set_segment_num(2);
        descriptor.set_sub_segment_num(154);
        descriptor.set_sub_segments_expected(201);

        splice.add_descriptor(descriptor.into());

        let expected_json: serde_json::Value = serde_json::from_str(
            r#"{
            "table_id": "0xfc",
            "section_syntax_indicator": false,
            "private_indicator": false,
            "sap_type": "0x3",
            "section_length": 54,
            "protocol_version": 0,
            "encrypted_packet": false,
            "encryption_algorithm": 0,
            "pts_adjustment": 0,
            "pts_adjustment_secs": 0.0,
            "pts_adjustment_human": "0s",
            "cw_index": "0xff",
            "tier": "0xfff",
            "splice_command_length": 5,
            "splice_command_type": 6,
            "splice_command_name": "TimeSignal",
            "splice_command": {
                "time_specified_flag": true,
                "pts_time": 1924989008,
                "pts_time_secs": 21388.766756
            },
            "descriptor_loop_length": 32,
            "descriptors": [
                {
                    "name": "Segmentation Descriptor",
                    "splice_descriptor_tag": "0x02",
                    "descriptor_length": 30,
                    "identifier": "CUEI",
                    "segmentation_event_id": "0x4800008e",
                    "segmentation_event_cancel_indicator": false,
                    "program_segmentation_flag": true,
                    "segmentation_duration_flag": true,
                    "delivery_not_restricted_flag": false,
                    "web_delivery_allowed_flag": false,
                    "no_regional_blackout_flag": true,
                    "archive_allowed_flag": true,
                    "device_restrictions": "None",
                    "components": [],
                    "segmentation_duration": 27630000,
                    "segmentation_duration_secs": 307.0,
                    "segmentation_duration_human": "5m 7s",
                    "segmentation_upid_type": "0x08",
                    "segmentation_upid_type_name": "AiringID",
                    "segmentation_upid_length": 8,
                    "segmentation_upid": "0x2ca0a18a",
                    "segmentation_message": "Provider Placement Opportunity Start",
                    "segmentation_type_id": 52,
                    "segment_num": 2,
                    "segments_expected": 0,
                    "sub_segment_num": 154,
                    "sub_segments_expected": 201
                }
            ],
            "crc_32": "0xb6c1a0f1"
        }"#,
        )?;

        assert_json_eq!(
            serde_json::to_value(&splice.into_encoded()?)?,
            expected_json
        );

        Ok(())
    }

    #[test]
    fn serialize_info_section_as_json() -> Result<()> {
        let splice = SpliceInfoSection::new(SpliceNull::default());

        assert_json_eq!(
            serde_json::to_value(&splice.into_encoded()?)?,
            serde_json::json!({
                "table_id": "0xfc",
                "section_syntax_indicator": false,
                "private_indicator": false,
                "sap_type": "0x3",
                "section_length": 17,
                "protocol_version": 0,
                "encrypted_packet": false,
                "encryption_algorithm": 0,
                "pts_adjustment": 0,
                "pts_adjustment_secs": 0.0,
                "pts_adjustment_human": "0s",
                "cw_index": "0x0",
                "tier": "0xfff",
                "splice_command_length": 0,
                "splice_command_type": 0,
                "splice_command_name": "SpliceNull",
                "splice_command": {},
                "descriptor_loop_length": 0,
                "descriptors": [],
                "crc_32": "0x7a4fbfff"
            })
        );

        Ok(())
    }
}
