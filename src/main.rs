use clap::{Parser, ValueEnum};
use data_encoding::BASE64;
use scte35::{
    SpliceCommand, SpliceDescriptor, SpliceInfoSection, parse_splice_info_section,
    validate_scte35_crc,
};
use std::process;

#[derive(Debug, Clone, ValueEnum, Default)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Parser)]
#[command(name = "scte35")]
#[command(about = "Parse SCTE-35 messages from base64-encoded payloads")]
#[command(version)]
struct Arguments {
    /// Base64-encoded SCTE-35 payload
    #[arg(value_name = "PAYLOAD")]
    payload: String,

    /// Output format
    #[arg(short = 'o', long = "output", value_enum, default_value_t = OutputFormat::Text)]
    output: OutputFormat,
}

fn print_text_output(section: &SpliceInfoSection, buffer: &[u8]) {
    println!("Successfully parsed SpliceInfoSection:");
    println!("  Table ID: {}", section.table_id);
    println!("  Section Length: {}", section.section_length);
    println!("  Protocol Version: {}", section.protocol_version);
    println!("  Splice Command Type: {}", section.splice_command_type);
    println!("  Splice Command Length: {}", section.splice_command_length);

    match &section.splice_command {
        SpliceCommand::SpliceNull => {
            println!("  Splice Command: SpliceNull");
        }
        SpliceCommand::SpliceSchedule(cmd) => {
            println!("  Splice Command: SpliceSchedule");
            println!("    Splice Event ID: {}", cmd.splice_event_id);
            println!("    Duration Flag: {}", cmd.duration_flag);
            if let Some(duration) = cmd.splice_duration {
                println!("    Splice Duration: {duration}");
            }
            if let Some(time) = cmd.utc_splice_time {
                println!("    UTC Splice Time: {time} (seconds since epoch)");
            }
        }
        SpliceCommand::SpliceInsert(cmd) => {
            println!("  Splice Command: SpliceInsert");
            println!("    Splice Event ID: 0x{:08x}", cmd.splice_event_id);
            println!(
                "    Splice Event Cancel: {}",
                cmd.splice_event_cancel_indicator
            );
            println!("    Out of Network: {}", cmd.out_of_network_indicator);
            println!("    Program Splice Flag: {}", cmd.program_splice_flag);
            println!("    Duration Flag: {}", cmd.duration_flag);
            println!("    Splice Immediate Flag: {}", cmd.splice_immediate_flag);

            if let Some(splice_time) = &cmd.splice_time {
                if let Some(pts) = splice_time.pts_time {
                    println!("    Splice Time PTS: 0x{pts:09x}");
                    if let Some(duration) = splice_time.to_duration() {
                        println!("    Splice Time: {:.6} seconds", duration.as_secs_f64());
                    }
                }
            }

            if let Some(break_duration) = &cmd.break_duration {
                println!("    Break Duration:");
                println!("      Auto Return: {}", break_duration.auto_return);
                let duration = break_duration.to_duration();
                println!(
                    "      Duration: 0x{:09x} ({:.6} seconds)",
                    break_duration.duration,
                    duration.as_secs_f64()
                );
            }

            println!("    Unique Program ID: {}", cmd.unique_program_id);
            println!("    Avail Num: {}", cmd.avail_num);
            println!("    Avails Expected: {}", cmd.avails_expected);
        }
        SpliceCommand::TimeSignal(cmd) => {
            println!("  Splice Command: TimeSignal");
            if let Some(pts) = cmd.splice_time.pts_time {
                println!("    PTS Time: {pts}");
            }
        }
        SpliceCommand::BandwidthReservation(cmd) => {
            println!("  Splice Command: BandwidthReservation");
            println!("    Bandwidth Reservation: {}", cmd.dwbw_reservation);
        }
        SpliceCommand::PrivateCommand(cmd) => {
            println!("  Splice Command: PrivateCommand");
            println!("    Private Command ID: {}", cmd.private_command_id);
            println!("    Private Command Length: {}", cmd.private_command_length);
        }
        SpliceCommand::Unknown => {
            println!("  Splice Command: Unknown");
        }
    }

    println!(
        "  Descriptor Loop Length: {}",
        section.descriptor_loop_length
    );
    println!(
        "  Number of Descriptors: {}",
        section.splice_descriptors.len()
    );
    for descriptor in &section.splice_descriptors {
        match descriptor {
            SpliceDescriptor::Segmentation(seg_desc) => {
                println!("    Segmentation Descriptor:");
                println!("      Event ID: 0x{:08x}", seg_desc.segmentation_event_id);
                println!(
                    "      Cancel Indicator: {}",
                    seg_desc.segmentation_event_cancel_indicator
                );
                println!(
                    "      Program Segmentation: {}",
                    seg_desc.program_segmentation_flag
                );
                println!(
                    "      Duration Flag: {}",
                    seg_desc.segmentation_duration_flag
                );

                if let Some(duration) = seg_desc.duration() {
                    println!("      Duration: {:.3} seconds", duration.as_secs_f64());
                }

                println!(
                    "      UPID Type: {} (0x{:02x})",
                    seg_desc.upid_type_description(),
                    u8::from(seg_desc.segmentation_upid_type)
                );
                println!(
                    "      UPID Length: {} bytes",
                    seg_desc.segmentation_upid_length
                );

                if let Some(upid_str) = seg_desc.upid_as_string() {
                    println!("      UPID: {upid_str}");
                } else if !seg_desc.segmentation_upid.is_empty() {
                    // Show base64 for binary data when base64 is available
                    #[cfg(feature = "base64")]
                    {
                        println!(
                            "      UPID (base64): {}",
                            BASE64.encode(&seg_desc.segmentation_upid)
                        );
                    }
                    #[cfg(not(feature = "base64"))]
                    {
                        println!(
                            "      UPID (hex): {}",
                            seg_desc
                                .segmentation_upid
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<_>>()
                                .join("")
                        );
                    }
                }

                println!(
                    "      Segmentation Type ID: 0x{:02x}",
                    seg_desc.segmentation_type_id
                );
                println!("      Segment Number: {}", seg_desc.segment_num);
                println!("      Segments Expected: {}", seg_desc.segments_expected);

                if let Some(sub_num) = seg_desc.sub_segment_num {
                    println!("      Sub-segment Number: {sub_num}");
                }
                if let Some(sub_expected) = seg_desc.sub_segments_expected {
                    println!("      Sub-segments Expected: {sub_expected}");
                }
            }
            SpliceDescriptor::Avail(avail_desc) => {
                println!("    Avail Descriptor:");
                println!("      Identifier: 0x{:08x}", avail_desc.identifier);
                println!(
                    "      Provider Avail ID: {} bytes",
                    avail_desc.provider_avail_id.len()
                );
            }
            SpliceDescriptor::Dtmf(dtmf_desc) => {
                println!("    DTMF Descriptor:");
                println!("      Identifier: 0x{:08x}", dtmf_desc.identifier);
                println!("      Preroll: {}", dtmf_desc.preroll);
                println!("      DTMF Count: {}", dtmf_desc.dtmf_count);
                let dtmf_chars: String = dtmf_desc
                    .dtmf_chars
                    .iter()
                    .map(|&c| if c.is_ascii_graphic() { c as char } else { '?' })
                    .collect();
                println!("      DTMF Characters: \"{dtmf_chars}\"");
            }
            SpliceDescriptor::Time(time_desc) => {
                println!("    Time Descriptor:");
                println!("      Identifier: 0x{:08x}", time_desc.identifier);
                println!("      TAI Seconds: {} bytes", time_desc.tai_seconds.len());
                println!("      TAI Nanoseconds: {} bytes", time_desc.tai_ns.len());
                println!("      UTC Offset: {} bytes", time_desc.utc_offset.len());
            }
            SpliceDescriptor::Audio(audio_desc) => {
                println!("    Audio Descriptor:");
                println!("      Identifier: 0x{:08x}", audio_desc.identifier);
                println!(
                    "      Audio Components: {} bytes",
                    audio_desc.audio_components.len()
                );
            }
            SpliceDescriptor::Unknown { tag, length, data } => {
                println!("    Unknown Descriptor:");
                println!("      Tag: 0x{tag:02x}");
                println!("      Length: {length}");
                if let Some(text) = descriptor.as_str() {
                    println!("      Content: \"{text}\"");
                } else {
                    println!("      Data: {} bytes", data.len());
                }
            }
        }
    }

    if let Some(crc) = section.e_crc_32 {
        println!("  Encrypted CRC-32: 0x{crc:08X}");
    }

    // CRC validation is always available when CLI feature is enabled
    // since cli feature depends on crc-validation
    match validate_scte35_crc(buffer) {
        Ok(true) => println!("  CRC-32: 0x{:08X} ✓ (Valid)", section.crc_32),
        Ok(false) => println!("  CRC-32: 0x{:08X} ✗ (Invalid)", section.crc_32),
        Err(e) => println!("  CRC-32: 0x{:08X} ✗ (Error: {})", section.crc_32, e),
    }
}

fn print_json_output(section: &SpliceInfoSection, buffer: &[u8]) {
    use serde_json::json;

    let crc_validation = match validate_scte35_crc(buffer) {
        Ok(valid) => json!({
            "valid": valid,
            "error": null
        }),
        Err(e) => json!({
            "valid": false,
            "error": e.to_string()
        }),
    };

    let output = json!({
        "status": "success",
        "data": section,
        "crc_validation": crc_validation
    });

    match serde_json::to_string_pretty(&output) {
        Ok(json_str) => println!("{json_str}"),
        Err(e) => {
            eprintln!("Error serializing to JSON: {e}");
            process::exit(1);
        }
    }
}

fn main() {
    let args = Arguments::parse();

    let base64_payload = &args.payload;

    let buffer = match BASE64.decode(base64_payload.as_bytes()) {
        Ok(data) => data,
        Err(e) => match args.output {
            OutputFormat::Text => {
                eprintln!("Error decoding base64 string: {e}");
                process::exit(1);
            }
            OutputFormat::Json => {
                use serde_json::json;
                let output = json!({
                    "status": "error",
                    "error": format!("Error decoding base64 string: {e}")
                });
                match serde_json::to_string_pretty(&output) {
                    Ok(json_str) => println!("{json_str}"),
                    Err(json_err) => {
                        eprintln!("Error serializing error to JSON: {json_err}");
                        process::exit(1);
                    }
                }
                process::exit(1);
            }
        },
    };

    match parse_splice_info_section(&buffer) {
        Ok(section) => match args.output {
            OutputFormat::Text => print_text_output(&section, &buffer),
            OutputFormat::Json => print_json_output(&section, &buffer),
        },
        Err(e) => match args.output {
            OutputFormat::Text => {
                eprintln!("Error parsing SpliceInfoSection: {e}");
                process::exit(1);
            }
            OutputFormat::Json => {
                use serde_json::json;
                let output = json!({
                    "status": "error",
                    "error": e.to_string()
                });
                match serde_json::to_string_pretty(&output) {
                    Ok(json_str) => println!("{json_str}"),
                    Err(json_err) => {
                        eprintln!("Error serializing error to JSON: {json_err}");
                        process::exit(1);
                    }
                }
                process::exit(1);
            }
        },
    }
}
