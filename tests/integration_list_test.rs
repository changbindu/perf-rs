//! Integration tests for the `perf list` command.

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
