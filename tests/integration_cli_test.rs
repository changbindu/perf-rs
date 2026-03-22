//! Integration tests for general CLI functionality.

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
