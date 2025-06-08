use base64::{engine::general_purpose, Engine};
use scte35_parsing::{
    parse_splice_info_section, validate_scte35_crc, SpliceCommand, SpliceDescriptor,
};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <base64-encoded-scte35-payload>", args[0]);
        eprintln!("\nExample:");
        eprintln!(
            "  {} /DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=",
            args[0]
        );
        process::exit(1);
    }

    let base64_payload = &args[1];

    let buffer = match general_purpose::STANDARD.decode(base64_payload) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error decoding base64 string: {}", e);
            process::exit(1);
        }
    };

    match parse_splice_info_section(&buffer) {
        Ok(section) => {
            println!("Successfully parsed SpliceInfoSection:");
            println!("  Table ID: {}", section.table_id);
            println!("  Section Length: {}", section.section_length);
            println!("  Protocol Version: {}", section.protocol_version);
            println!("  Splice Command Type: {}", section.splice_command_type);
            println!("  Splice Command Length: {}", section.splice_command_length);

            match section.splice_command {
                SpliceCommand::SpliceNull => {
                    println!("  Splice Command: SpliceNull");
                }
                SpliceCommand::SpliceSchedule(cmd) => {
                    println!("  Splice Command: SpliceSchedule");
                    println!("    Splice Event ID: {}", cmd.splice_event_id);
                    println!("    Duration Flag: {}", cmd.duration_flag);
                    if let Some(duration) = cmd.splice_duration {
                        println!("    Splice Duration: {}", duration);
                    }
                    if let Some(time) = cmd.scheduled_splice_time {
                        println!("    Scheduled Splice Time: {:?}", time);
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
                            println!(
                                "    Splice Time PTS: 0x{:09x} ({:.6} seconds)",
                                pts,
                                pts as f64 / 90000.0
                            );
                        }
                    }

                    if let Some(break_duration) = &cmd.break_duration {
                        println!("    Break Duration:");
                        println!("      Auto Return: {}", break_duration.auto_return);
                        println!(
                            "      Duration: 0x{:09x} ({:.6} seconds)",
                            break_duration.duration,
                            break_duration.duration as f64 / 90000.0
                        );
                    }

                    println!("    Unique Program ID: {}", cmd.unique_program_id);
                    println!("    Avail Num: {}", cmd.avail_num);
                    println!("    Avails Expected: {}", cmd.avails_expected);
                }
                SpliceCommand::TimeSignal(cmd) => {
                    println!("  Splice Command: TimeSignal");
                    if let Some(pts) = cmd.splice_time.pts_time {
                        println!("    PTS Time: {}", pts);
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

                        if let Some(duration) = seg_desc.segmentation_duration {
                            println!("      Duration: {:.3} seconds", duration as f64 / 90000.0);
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
                            println!("      UPID: {}", upid_str);
                        } else if !seg_desc.segmentation_upid.is_empty() {
                            // Show base64 for binary data when base64 is available
                            #[cfg(feature = "base64")]
                            {
                                println!(
                                    "      UPID (base64): {}",
                                    general_purpose::STANDARD.encode(&seg_desc.segmentation_upid)
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
                            println!("      Sub-segment Number: {}", sub_num);
                        }
                        if let Some(sub_expected) = seg_desc.sub_segments_expected {
                            println!("      Sub-segments Expected: {}", sub_expected);
                        }
                    }
                    SpliceDescriptor::Unknown { tag, length, data } => {
                        println!("    Unknown Descriptor:");
                        println!("      Tag: 0x{:02x}", tag);
                        println!("      Length: {}", length);
                        if let Some(text) = descriptor.as_str() {
                            println!("      Content: \"{}\"", text);
                        } else {
                            println!("      Data: {} bytes", data.len());
                        }
                    }
                }
            }

            if let Some(crc) = section.e_crc_32 {
                println!("  Encrypted CRC-32: 0x{:08X}", crc);
            }

            // CRC validation is always available when CLI feature is enabled
            // since cli feature depends on crc-validation
            match validate_scte35_crc(&buffer) {
                Ok(true) => println!("  CRC-32: 0x{:08X} ✓ (Valid)", section.crc_32),
                Ok(false) => println!("  CRC-32: 0x{:08X} ✗ (Invalid)", section.crc_32),
                Err(e) => println!("  CRC-32: 0x{:08X} ✗ (Error: {})", section.crc_32, e),
            }
        }
        Err(e) => {
            eprintln!("Error parsing SpliceInfoSection: {}", e);
            process::exit(1);
        }
    }
}
