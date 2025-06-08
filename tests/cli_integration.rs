//! Integration tests for CLI functionality to validate README examples

#[cfg(feature = "cli")]
#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use predicates::prelude::*;

    // Base64 payload used in README examples
    const README_EXAMPLE_PAYLOAD: &str =
        "/DAvAAAAAAAA///wFAVIAACPf+/+c2nALv4AUsz1AAAAAAAKAAhDVUVJAAABNWLbowo=";

    #[test]
    fn test_cli_text_output_works() {
        let mut cmd = Command::cargo_bin("scte35-parsing").unwrap();
        let output = cmd
            .arg(README_EXAMPLE_PAYLOAD)
            .output()
            .expect("Failed to execute CLI command");

        assert!(output.status.success(), "CLI command should succeed");

        let stdout = String::from_utf8(output.stdout).expect("Output should be valid UTF-8");

        // Verify key elements from README are present
        assert!(stdout.contains("Successfully parsed SpliceInfoSection"));
        assert!(stdout.contains("Table ID: 252"));
        assert!(stdout.contains("Splice Command: SpliceInsert"));
        assert!(stdout.contains("Splice Event ID: 0x4800008f"));
        assert!(stdout.contains("seconds"));
        assert!(stdout.contains("CRC-32:"));
    }

    #[test]
    fn test_cli_json_output_works() {
        let mut cmd = Command::cargo_bin("scte35-parsing").unwrap();
        let output = cmd
            .args(&["-o", "json", README_EXAMPLE_PAYLOAD])
            .output()
            .expect("Failed to execute CLI command");

        assert!(output.status.success(), "CLI command should succeed");

        let stdout = String::from_utf8(output.stdout).expect("Output should be valid UTF-8");
        let json: serde_json::Value =
            serde_json::from_str(&stdout).expect("Output should be valid JSON");

        // Verify JSON structure matches README examples
        assert_eq!(json["status"], "success");
        assert_eq!(json["data"]["table_id"], 252);
        assert_eq!(json["data"]["splice_command"]["type"], "SpliceInsert");
        assert_eq!(json["data"]["splice_command"]["splice_event_id"], 1207959695);
        assert!(json["crc_validation"]["valid"].as_bool().unwrap());
    }

    #[test]
    fn test_cli_json_output_long_flag() {
        let mut cmd = Command::cargo_bin("scte35-parsing").unwrap();
        let output = cmd
            .args(&["--output", "json", README_EXAMPLE_PAYLOAD])
            .output()
            .expect("Failed to execute CLI command");

        assert!(output.status.success(), "CLI command should succeed");

        let stdout = String::from_utf8(output.stdout).expect("Output should be valid UTF-8");
        let _json: serde_json::Value =
            serde_json::from_str(&stdout).expect("Output should be valid JSON");
    }

    #[test]
    fn test_cli_help_contains_expected_text() {
        let mut cmd = Command::cargo_bin("scte35-parsing").unwrap();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Parse SCTE-35 messages from base64-encoded payloads",
            ))
            .stdout(predicate::str::contains("Base64-encoded SCTE-35 payload"))
            .stdout(predicate::str::contains("Output format"));
    }

    #[test]
    fn test_cli_version_works() {
        let mut cmd = Command::cargo_bin("scte35-parsing").unwrap();
        cmd.arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::starts_with("scte35-parsing"))
            .stderr("");
    }

    #[test]
    fn test_cli_handles_invalid_base64() {
        let mut cmd = Command::cargo_bin("scte35-parsing").unwrap();
        cmd.arg("invalid_base64!")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Error decoding base64 string"));
    }

    #[test]
    fn test_cli_handles_invalid_base64_json() {
        let mut cmd = Command::cargo_bin("scte35-parsing").unwrap();
        let output = cmd
            .args(&["-o", "json", "invalid_base64!"])
            .output()
            .expect("Failed to execute CLI command");

        assert!(!output.status.success(), "CLI command should fail");

        let stdout = String::from_utf8(output.stdout).expect("Output should be valid UTF-8");
        let json: serde_json::Value =
            serde_json::from_str(&stdout).expect("Error output should be valid JSON");

        // Verify error structure
        assert_eq!(json["status"], "error");
        assert!(json["error"]
            .as_str()
            .unwrap()
            .contains("Error decoding base64 string"));
    }
}