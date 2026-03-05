//! Integration tests for perf-rs CLI commands.
//! Tests that require root privileges are skipped automatically.

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

/// Check if we have permission to run perf events
fn has_perf_permission() -> bool {
    // Either root, CAP_SYS_ADMIN, or perf_event_paranoid <= 0
    if unsafe { libc::getuid() } == 0 {
        return true;
    }

    if let Ok(content) = std::fs::read_to_string("/proc/sys/kernel/perf_event_paranoid") {
        if let Ok(level) = content.trim().parse::<i32>() {
            return level <= 0;
        }
    }

    false
}

#[test]
fn test_list_no_args() {
    let (success, stdout, stderr) = run_perf(&["list"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(stdout.contains("Hardware event") || stdout.contains("cpu-cycles"));
    assert!(stdout.contains("instructions"));
}

#[test]
fn test_list_with_filter() {
    let (success, stdout, stderr) = run_perf(&["list", "--filter", "cache"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(stdout.contains("cache"));
}

#[test]
fn test_list_with_detailed() {
    let (success, stdout, stderr) = run_perf(&["list", "--detailed"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(!stdout.is_empty());
}

#[test]
fn test_list_filter_no_match() {
    let (success, stdout, _stderr) = run_perf(&["list", "--filter", "nonexistent_xyz"]);

    assert!(success);
    assert!(
        stdout.contains("No events found") || stdout.is_empty() || !stdout.contains("cpu-cycles")
    );
}

#[test]
fn test_stat_no_command() {
    let (success, _stdout, stderr) = run_perf(&["stat"]);

    assert!(!success);
    assert!(stderr.contains("No command specified") || stderr.contains("error"));
}

#[test]
fn test_stat_simple_command() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let (success, stdout, stderr) = run_perf(&["stat", "--", "echo", "hello"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(stdout.contains("cpu-cycles") || stdout.contains("instructions"));
}

#[test]
fn test_stat_with_event() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let (success, stdout, stderr) = run_perf(&["stat", "-e", "instructions", "--", "true"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(stdout.contains("instructions"));
}

#[test]
fn test_stat_invalid_event() {
    let (success, _stdout, stderr) = run_perf(&["stat", "-e", "invalid_event_xyz", "--", "true"]);

    assert!(!success);
    assert!(stderr.contains("Unknown event") || stderr.contains("error"));
}

#[test]
fn test_stat_multiple_events() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let (success, stdout, stderr) =
        run_perf(&["stat", "-e", "cpu-cycles,instructions", "--", "true"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(stdout.contains("cpu-cycles"));
    assert!(stdout.contains("instructions"));
}

#[test]
fn test_record_no_command() {
    let (success, _stdout, stderr) = run_perf(&["record"]);

    assert!(!success);
    assert!(stderr.contains("No command specified") || stderr.contains("error"));
}

#[test]
fn test_record_simple_command() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_perf.data");
    let output_arg = output_path.to_str().unwrap();

    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        output_path.exists(),
        "Output file not created: {:?}",
        output_path
    );

    let metadata = std::fs::metadata(&output_path).expect("Failed to read output file metadata");
    assert!(metadata.len() > 0, "Output file is empty");
}

#[test]
fn test_record_with_period() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_period.data");
    let output_arg = output_path.to_str().unwrap();

    let (success, _stdout, stderr) = run_perf(&[
        "record", "--output", output_arg, "--period", "1000000", "--", "true",
    ]);

    assert!(success, "Command failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");
}

#[test]
fn test_record_with_event() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_event.data");
    let output_arg = output_path.to_str().unwrap();

    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--event",
        "instructions",
        "--frequency",
        "99",
        "--",
        "true",
    ]);

    assert!(success, "Command failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");
}

#[test]
fn test_record_invalid_event() {
    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "-e",
        "invalid_event",
        "--frequency",
        "99",
        "--",
        "true",
    ]);

    assert!(!success);
    assert!(stderr.contains("Unknown event") || stderr.contains("error"));
}

#[test]
fn test_verbose_flag() {
    let (success, _stdout, stderr) = run_perf(&["--verbose", "list"]);

    assert!(success, "Command failed: {}", stderr);
}

#[test]
fn test_help_flag() {
    let result = Command::new("cargo")
        .args(["run", "--", "--help"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(result.status.success());
    assert!(stdout.contains("perf-rs"));
    assert!(stdout.contains("list") || stdout.contains("stat") || stdout.contains("record"));
}

#[test]
fn test_version_flag() {
    let result = Command::new("cargo")
        .args(["run", "--", "--version"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(result.status.success());
    assert!(stdout.contains("perf-rs"));
}

#[test]
fn test_stat_help() {
    let result = Command::new("cargo")
        .args(["run", "--", "stat", "--help"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(result.status.success());
    assert!(stdout.contains("stat"));
    assert!(stdout.contains("--pid") || stdout.contains("--event"));
}

#[test]
fn test_record_help() {
    let result = Command::new("cargo")
        .args(["run", "--", "record", "--help"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(result.status.success());
    assert!(stdout.contains("record"));
    assert!(stdout.contains("--output") || stdout.contains("--frequency"));
}

#[test]
fn test_list_help() {
    let result = Command::new("cargo")
        .args(["run", "--", "list", "--help"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(result.status.success());
    assert!(stdout.contains("list"));
    assert!(stdout.contains("--filter") || stdout.contains("--detailed"));
}

#[test]
fn test_stat_nonexistent_command() {
    let (success, _stdout, stderr) = run_perf(&["stat", "--", "nonexistent_command_xyz_123"]);

    assert!(!success);
    assert!(!stderr.is_empty());
}

#[test]
fn test_record_nonexistent_command() {
    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "--frequency",
        "99",
        "--",
        "nonexistent_command_xyz_123",
    ]);

    assert!(!success);
    assert!(!stderr.is_empty());
}
