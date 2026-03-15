//! Integration tests for pagination functionality.
//!
//! Tests pagination behavior in report, script, and list commands,
//! including --no-pager flag and piped output handling.

use std::process::{Command, Stdio};

/// Helper to run perf-rs with arguments
fn run_perf(args: &[&str]) -> (bool, String, String) {
    let result = Command::new("cargo")
        .args(["run", "--"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let success = result.status.success();

    (success, stdout, stderr)
}

/// Helper to run perf-rs with piped stdin (simulates non-TTY)
fn run_perf_piped(args: &[&str]) -> (bool, String, String) {
    let result = Command::new("cargo")
        .args(["run", "--"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let success = result.status.success();

    (success, stdout, stderr)
}

#[test]
fn test_pager_module_integration() {
    let (success, stdout, stderr) = run_perf(&["list"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(!stdout.is_empty(), "Expected output from list command");
}

#[test]
fn test_no_pager_flag_parsing() {
    let result = Command::new("cargo")
        .args(["run", "--", "--no-pager", "list", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    assert!(
        result.status.success(),
        "--no-pager flag should be accepted"
    );
}

#[test]
fn test_no_pager_flag_with_list() {
    let (success, stdout, stderr) = run_perf(&["--no-pager", "list"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        !stdout.contains("-- More --"),
        "Output should not contain pagination prompt with --no-pager"
    );
}

#[test]
fn test_no_pager_flag_position() {
    let (success1, _, _) = run_perf(&["--no-pager", "list"]);
    let (success2, _, _) = run_perf(&["list", "--no-pager"]);

    assert!(
        success1 || success2,
        "--no-pager should work in at least one position"
    );
}

#[test]
fn test_list_command_no_pager() {
    let (success, stdout, stderr) = run_perf(&["--no-pager", "list"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("Hardware")
            || stdout.contains("cpu-cycles")
            || stdout.contains("instructions"),
        "Output should contain event information"
    );
    assert!(
        !stdout.contains("-- More --"),
        "Output should not contain pagination prompt"
    );
}

#[test]
fn test_list_command_detailed_no_pager() {
    let (success, stdout, stderr) = run_perf(&["--no-pager", "list", "--detailed"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        !stdout.contains("-- More --"),
        "Output should not contain pagination prompt"
    );
}

#[test]
fn test_list_command_filter_no_pager() {
    let (success, stdout, stderr) = run_perf(&["--no-pager", "list", "--filter", "cache"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        !stdout.contains("-- More --"),
        "Output should not contain pagination prompt"
    );
}

#[test]
fn test_piped_output_disables_pagination() {
    let (success, stdout, stderr) = run_perf_piped(&["list"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        !stdout.contains("-- More --"),
        "Piped output should not contain pagination prompt"
    );
}

#[test]
fn test_piped_output_list_detailed() {
    let (success, stdout, stderr) = run_perf_piped(&["list", "--detailed"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        !stdout.contains("-- More --"),
        "Piped detailed output should not contain pagination prompt"
    );
}

#[test]
fn test_piped_output_list_filter() {
    let (success, stdout, stderr) = run_perf_piped(&["list", "--filter", "cycles"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        !stdout.contains("-- More --"),
        "Piped filtered output should not contain pagination prompt"
    );
}

#[test]
fn test_report_no_pager_flag() {
    let result = Command::new("cargo")
        .args(["run", "--", "--no-pager", "report", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    assert!(
        result.status.success(),
        "--no-pager flag should be accepted for report"
    );
}

#[test]
fn test_script_no_pager_flag() {
    let result = Command::new("cargo")
        .args(["run", "--", "--no-pager", "script", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    assert!(
        result.status.success(),
        "--no-pager flag should be accepted for script"
    );
}

#[test]
fn test_help_shows_no_pager_option() {
    let result = Command::new("cargo")
        .args(["run", "--", "--help"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(result.status.success());
    assert!(
        stdout.contains("--no-pager"),
        "Help should show --no-pager option"
    );
}

#[test]
fn test_list_help_shows_options() {
    let result = Command::new("cargo")
        .args(["run", "--", "list", "--help"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(result.status.success());
    assert!(stdout.contains("--filter") || stdout.contains("--detailed"));
}
