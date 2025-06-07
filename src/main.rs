use base64::{Engine, engine::general_purpose};
use scte35_parsing::{parse_splice_info_section, SpliceCommand};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <base64-encoded-scte35-payload>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} /DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=", args[0]);
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
                    println!("    Splice Event Cancel: {}", cmd.splice_event_cancel_indicator);
                    println!("    Out of Network: {}", cmd.out_of_network_indicator);
                    println!("    Program Splice Flag: {}", cmd.program_splice_flag);
                    println!("    Duration Flag: {}", cmd.duration_flag);
                    println!("    Splice Immediate Flag: {}", cmd.splice_immediate_flag);
                    
                    if let Some(splice_time) = &cmd.splice_time {
                        if let Some(pts) = splice_time.pts_time {
                            println!("    Splice Time PTS: 0x{:09x} ({:.6} seconds)", pts, pts as f64 / 90000.0);
                        }
                    }
                    
                    if let Some(break_duration) = &cmd.break_duration {
                        println!("    Break Duration:");
                        println!("      Auto Return: {}", break_duration.auto_return);
                        println!("      Duration: 0x{:09x} ({:.6} seconds)", break_duration.duration, break_duration.duration as f64 / 90000.0);
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
                println!("    Descriptor Tag: {}", descriptor.descriptor_tag);
                println!("    Descriptor Length: {}", descriptor.descriptor_length);
            }

            if let Some(crc) = section.e_crc_32 {
                println!("  Encrypted CRC-32: {}", crc);
            }
            println!("  CRC-32: {}", section.crc_32);
        }
        Err(e) => {
            eprintln!("Error parsing SpliceInfoSection: {}", e);
            process::exit(1);
        }
    }
}