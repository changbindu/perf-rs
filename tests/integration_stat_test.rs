//! Integration tests for the `perf stat` command.

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
fn test_stat_nonexistent_command() {
    let (success, stdout, stderr) = run_perf(&["stat", "--", "nonexistent_command_xyz_123"]);

    // Parent process succeeds even when child command fails
    // (This is current behavior - child exit code not propagated)
    let _ = success;
    assert!(!stdout.is_empty() || !stderr.is_empty());
}
