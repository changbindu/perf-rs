//! Integration tests for tracepoint support in stat and record commands.
//!
//! These tests verify the tracepoint functionality of perf-rs, including:
//! - Tracepoint event parsing and validation
//! - Stat command with tracepoint events
//! - Record command with tracepoint events
//! - Report command reading tracepoint samples
//! - Error handling for invalid tracepoints
//!
//! # Running Tests
//!
//! Tests that require root privileges are marked with `#[ignore]`.
//! To run all tests including ignored ones:
//!
//! ```bash
//! # Run tests that don't require root
//! cargo test --test tracepoint_integration
//!
//! # Run all tests including those requiring root
//! sudo cargo test --test tracepoint_integration -- --include-ignored
//! ```
//!
//! # Tracepoint Requirements
//!
//! Tracepoints require:
//! - Root access, or
//! - CAP_SYS_ADMIN capability, or
//! - Access to tracefs (/sys/kernel/tracing or /sys/kernel/debug/tracing)

use std::path::Path;
use std::process::{Command, Stdio};

/// Helper to run perf-rs with arguments.
/// Returns (success, stdout, stderr).
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

/// Check if we have permission to run perf events.
fn has_perf_permission() -> bool {
    // Check for root
    if unsafe { libc::getuid() } == 0 {
        return true;
    }

    // Check perf_event_paranoid setting
    if let Ok(content) = std::fs::read_to_string("/proc/sys/kernel/perf_event_paranoid") {
        if let Ok(level) = content.trim().parse::<i32>() {
            return level <= 0;
        }
    }

    false
}

/// Check if tracefs is accessible.
fn has_tracefs_access() -> bool {
    // Check primary tracefs path
    if Path::new("/sys/kernel/tracing").exists() {
        if let Ok(_) = std::fs::read_dir("/sys/kernel/tracing/events") {
            return true;
        }
    }

    // Check fallback tracefs path
    if Path::new("/sys/kernel/debug/tracing").exists() {
        if let Ok(_) = std::fs::read_dir("/sys/kernel/debug/tracing/events") {
            return true;
        }
    }

    false
}

/// Check if a specific tracepoint exists.
fn tracepoint_exists(subsystem: &str, name: &str) -> bool {
    let primary_path = format!("/sys/kernel/tracing/events/{}/{}/id", subsystem, name);
    let fallback_path = format!("/sys/kernel/debug/tracing/events/{}/{}/id", subsystem, name);

    Path::new(&primary_path).exists() || Path::new(&fallback_path).exists()
}

/// Clean up test artifacts.
fn cleanup_test_files() {
    let _ = std::fs::remove_file("test_tracepoint.data");
    let _ = std::fs::remove_file("test_invalid_tp.data");
}

// ============================================================================
// Tests that DO NOT require root privileges
// ============================================================================

#[test]
fn test_stat_help_shows_tracepoint_format() {
    let (success, stdout, stderr) = run_perf(&["stat", "--help"]);

    assert!(success, "Command failed: {}", stderr);
    // Help should show event option
    assert!(
        stdout.contains("--event") || stdout.contains("-e"),
        "Help should mention --event option"
    );
}

#[test]
fn test_record_help_shows_tracepoint_format() {
    let (success, stdout, stderr) = run_perf(&["record", "--help"]);

    assert!(success, "Command failed: {}", stderr);
    // Help should show event option
    assert!(
        stdout.contains("--event") || stdout.contains("-e"),
        "Help should mention --event option"
    );
}

#[test]
fn test_stat_invalid_tracepoint_format_no_colon() {
    // Tracepoint format requires subsystem:name
    let (success, stdout, stderr) = run_perf(&["stat", "-e", "invalid_no_colon", "--", "true"]);

    assert!(
        !success,
        "Command should fail with invalid tracepoint format"
    );
    let combined = format!("{}{}", stdout, stderr).to_lowercase();
    assert!(
        combined.contains("unknown")
            || combined.contains("error")
            || combined.contains("not found"),
        "Error message should mention unknown event or error. Got: {}",
        combined
    );
}

#[test]
fn test_stat_invalid_tracepoint_empty_subsystem() {
    // Empty subsystem should fail
    let (success, stdout, stderr) = run_perf(&["stat", "-e", ":event_name", "--", "true"]);

    assert!(!success, "Command should fail with empty subsystem");
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("error") || combined.contains("invalid") || combined.contains("Unknown"),
        "Error message should mention error. Got: {}",
        combined
    );
}

#[test]
fn test_stat_invalid_tracepoint_empty_name() {
    // Empty name should fail
    let (success, stdout, stderr) = run_perf(&["stat", "-e", "subsystem:", "--", "true"]);

    assert!(!success, "Command should fail with empty name");
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("error") || combined.contains("invalid") || combined.contains("Unknown"),
        "Error message should mention error. Got: {}",
        combined
    );
}

#[test]
fn test_stat_nonexistent_tracepoint() {
    // Nonexistent tracepoint should fail
    let (success, stdout, stderr) =
        run_perf(&["stat", "-e", "nonexistent:fake_event", "--", "true"]);

    // This test may pass if tracefs is not accessible (permission denied)
    // or fail if tracefs is accessible but tracepoint doesn't exist
    if has_tracefs_access() {
        assert!(!success, "Command should fail with nonexistent tracepoint");
        let combined = format!("{}{}", stdout, stderr).to_lowercase();
        assert!(
            combined.contains("not found")
                || combined.contains("error")
                || combined.contains("does not exist"),
            "Error message should mention tracepoint not found. Got: {}",
            combined
        );
    }
}

#[test]
fn test_record_invalid_tracepoint_format_no_colon() {
    // Tracepoint format requires subsystem:name
    let (success, stdout, stderr) = run_perf(&["record", "-e", "invalid_no_colon", "--", "true"]);

    assert!(
        !success,
        "Command should fail with invalid tracepoint format"
    );
    let combined = format!("{}{}", stdout, stderr).to_lowercase();
    assert!(
        combined.contains("unknown")
            || combined.contains("error")
            || combined.contains("not found"),
        "Error message should mention unknown event or error. Got: {}",
        combined
    );
}

#[test]
fn test_record_nonexistent_tracepoint() {
    // Nonexistent tracepoint should fail
    let (success, stdout, stderr) =
        run_perf(&["record", "-e", "nonexistent:fake_event", "--", "true"]);

    if has_tracefs_access() {
        assert!(!success, "Command should fail with nonexistent tracepoint");
        let combined = format!("{}{}", stdout, stderr).to_lowercase();
        assert!(
            combined.contains("not found")
                || combined.contains("error")
                || combined.contains("does not exist"),
            "Error message should mention tracepoint not found. Got: {}",
            combined
        );
    }
}

#[test]
fn test_list_shows_tracepoint_header() {
    // perf list should show tracepoint section if tracefs is accessible
    let (success, stdout, stderr) = run_perf(&["list"]);

    assert!(success, "Command failed: {}", stderr);

    // Check if tracepoints are actually shown in the output
    // This depends on both tracefs access AND permission to read tracepoint events
    if stdout.contains("tracepoint") || stdout.contains("Tracepoint") {
        // Tracepoints are shown - test passes
        return;
    }

    // If tracepoints aren't shown, verify the command still succeeds
    // (tracepoint listing may fail silently due to permissions)
    assert!(
        stdout.contains("hardware") || stdout.contains("Hardware"),
        "Output should at least show hardware events. Got: {}",
        stdout
    );
}

// ============================================================================
// Tests that require root privileges (marked with #[ignore])
// ============================================================================

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_stat_with_sched_switch_tracepoint() {
    cleanup_test_files();

    if !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: sched:sched_switch tracepoint not found");
        return;
    }

    let (success, stdout, stderr) =
        run_perf(&["stat", "-e", "sched:sched_switch", "--", "sleep", "0.1"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("sched:sched_switch") || stdout.contains("sched_switch"),
        "Output should contain tracepoint name. Got: {}",
        stdout
    );
}

#[test]
fn test_stat_tracepoint_counts_events() {
    if !has_tracefs_access() || !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: tracefs not accessible or sched:sched_switch not found");
        return;
    }

    let (success, stdout, stderr) =
        run_perf(&["stat", "-e", "sched:sched_switch", "--", "sleep", "1"]);

    assert!(success, "Command failed: {}", stderr);

    // Parse the count from output
    // Output format: "                 N  sched:sched_switch"
    let count = stdout
        .lines()
        .find(|line| line.contains("sched:sched_switch") || line.contains("sched_switch"))
        .and_then(|line| {
            line.split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
        });

    assert!(
        count.is_some(),
        "Could not parse count from output: {}",
        stdout
    );
    assert!(
        count.unwrap() > 0,
        "Tracepoint count should be > 0, got: {}",
        count.unwrap()
    );
}

#[test]
fn test_record_tracepoint_captures_samples() {
    if !has_tracefs_access() || !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: tracefs not accessible or sched:sched_switch not found");
        return;
    }

    let _ = std::fs::remove_file("test_tracepoint_count.data");

    let (success, stdout, stderr) = run_perf(&[
        "record",
        "-e",
        "sched:sched_switch",
        "-o",
        "test_tracepoint_count.data",
        "--",
        "sleep",
        "1",
    ]);

    assert!(success, "Command failed: {}", stderr);

    // Parse the sample count from stderr (record output goes to stderr)
    // Output format: "Recorded N samples in X.XXs (Y lost)"
    let combined_output = format!("{} {}", stdout, stderr);
    let sample_count = combined_output
        .lines()
        .find(|line| line.contains("Recorded"))
        .and_then(|line| {
            line.split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u64>().ok())
        });

    assert!(
        sample_count.is_some(),
        "Could not parse sample count from output: {}",
        combined_output
    );
    assert!(
        sample_count.unwrap() > 0,
        "Recorded samples should be > 0, got: {}",
        sample_count.unwrap()
    );

    let _ = std::fs::remove_file("test_tracepoint_count.data");
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_stat_with_syscalls_tracepoint() {
    cleanup_test_files();

    // Use a common syscall tracepoint
    if !tracepoint_exists("syscalls", "sys_enter_openat") {
        eprintln!("Skipping test: syscalls:sys_enter_openat tracepoint not found");
        return;
    }

    let (success, stdout, stderr) = run_perf(&[
        "stat",
        "-e",
        "syscalls:sys_enter_openat",
        "--",
        "ls",
        "/tmp",
    ]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("syscalls:sys_enter_openat") || stdout.contains("sys_enter_openat"),
        "Output should contain tracepoint name. Got: {}",
        stdout
    );
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_stat_with_mixed_events_tracepoint_and_hardware() {
    cleanup_test_files();

    if !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: sched:sched_switch tracepoint not found");
        return;
    }

    // Mix hardware event with tracepoint
    let (success, stdout, stderr) = run_perf(&[
        "stat",
        "-a",
        "-e",
        "cpu-cycles,sched:sched_switch",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("cpu-cycles"),
        "Output should contain cpu-cycles. Got: {}",
        stdout
    );
    assert!(
        stdout.contains("sched:sched_switch") || stdout.contains("sched_switch"),
        "Output should contain tracepoint name. Got: {}",
        stdout
    );
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_record_with_sched_switch_tracepoint() {
    cleanup_test_files();

    if !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: sched:sched_switch tracepoint not found");
        return;
    }

    let (success, stdout, stderr) = run_perf(&[
        "record",
        "-e",
        "sched:sched_switch",
        "-o",
        "test_tracepoint.data",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        Path::new("test_tracepoint.data").exists(),
        "Output file should be created"
    );
    assert!(
        stdout.contains("Recorded") || stderr.contains("Recorded"),
        "Output should mention recording. Got stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    cleanup_test_files();
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_record_with_syscalls_tracepoint() {
    cleanup_test_files();

    if !tracepoint_exists("syscalls", "sys_enter_openat") {
        eprintln!("Skipping test: syscalls:sys_enter_openat tracepoint not found");
        return;
    }

    let (success, stdout, stderr) = run_perf(&[
        "record",
        "-e",
        "syscalls:sys_enter_openat",
        "-o",
        "test_tracepoint.data",
        "--",
        "ls",
        "/tmp",
    ]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        Path::new("test_tracepoint.data").exists(),
        "Output file should be created"
    );

    cleanup_test_files();
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_report_reads_tracepoint_data() {
    cleanup_test_files();

    if !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: sched:sched_switch tracepoint not found");
        return;
    }

    // First record with tracepoint
    let (record_success, _record_stdout, record_stderr) = run_perf(&[
        "record",
        "-e",
        "sched:sched_switch",
        "-o",
        "test_tracepoint.data",
        "--",
        "sleep",
        "0.2",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);

    // Then report the data
    let (report_success, report_stdout, report_stderr) =
        run_perf(&["report", "-i", "test_tracepoint.data"]);

    assert!(report_success, "Report failed: {}", report_stderr);
    assert!(
        report_stdout.contains("Samples") || report_stdout.contains("samples"),
        "Report should show sample count. Got: {}",
        report_stdout
    );

    cleanup_test_files();
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_stat_tracepoint_permission_denied_without_root() {
    // This test verifies that non-root users get a clear error
    // It will pass either by:
    // 1. Having no privileges (command fails with permission error)
    // 2. Having privileges (command succeeds - we skip the assertion)
    if has_perf_permission() && has_tracefs_access() {
        eprintln!("Skipping test: running with sufficient privileges");
        return;
    }

    if !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: sched:sched_switch tracepoint not found");
        return;
    }

    let (success, stdout, stderr) =
        run_perf(&["stat", "-e", "sched:sched_switch", "--", "sleep", "0.1"]);

    if !has_perf_permission() {
        assert!(!success, "Command should fail without privileges");
        let combined = format!("{}{}", stdout, stderr).to_lowercase();
        assert!(
            combined.contains("privilege")
                || combined.contains("permission")
                || combined.contains("denied")
                || combined.contains("cap_sys_admin")
                || combined.contains("cap_perfmon"),
            "Error message should mention privileges or permissions. Got: {}",
            combined
        );
    }
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_record_tracepoint_permission_denied_without_root() {
    cleanup_test_files();

    if has_perf_permission() && has_tracefs_access() {
        eprintln!("Skipping test: running with sufficient privileges");
        return;
    }

    if !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: sched:sched_switch tracepoint not found");
        return;
    }

    let (success, stdout, stderr) = run_perf(&[
        "record",
        "-e",
        "sched:sched_switch",
        "-o",
        "test_tracepoint.data",
        "--",
        "sleep",
        "0.1",
    ]);

    if !has_perf_permission() {
        assert!(!success, "Command should fail without privileges");
        let combined = format!("{}{}", stdout, stderr).to_lowercase();
        assert!(
            combined.contains("privilege")
                || combined.contains("permission")
                || combined.contains("denied"),
            "Error message should mention privileges or permissions. Got: {}",
            combined
        );
    }

    cleanup_test_files();
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_stat_multiple_tracepoints() {
    cleanup_test_files();

    let has_sched = tracepoint_exists("sched", "sched_switch");
    let has_syscalls = tracepoint_exists("syscalls", "sys_enter_openat");

    if !has_sched && !has_syscalls {
        eprintln!("Skipping test: no common tracepoints found");
        return;
    }

    let mut events = Vec::new();
    if has_sched {
        events.push("sched:sched_switch");
    }
    if has_syscalls {
        events.push("syscalls:sys_enter_openat");
    }

    let events_str = events.join(",");
    let args: Vec<&str> = vec!["stat", "-e", &events_str, "--", "sleep", "0.1"];
    let args: Vec<&str> = args.into_iter().collect();

    let (success, stdout, stderr) = run_perf(&args);

    assert!(success, "Command failed: {}", stderr);

    for event in &events {
        assert!(
            stdout.contains(event) || stdout.contains(&event.split(':').nth(1).unwrap_or("")),
            "Output should contain tracepoint {}. Got: {}",
            event,
            stdout
        );
    }
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_list_tracepoints_by_subsystem() {
    let (success, stdout, stderr) = run_perf(&["list"]);

    assert!(success, "Command failed: {}", stderr);

    if has_tracefs_access() {
        // Check that tracepoints are grouped by subsystem
        assert!(
            stdout.contains("sched:") || stdout.contains("sched\n"),
            "Output should show sched subsystem. Got: {}",
            stdout
        );
    }
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_list_filter_tracepoints() {
    let (success, stdout, stderr) = run_perf(&["list", "--filter", "sched"]);

    assert!(success, "Command failed: {}", stderr);

    if has_tracefs_access() {
        // Filter should show only sched-related events
        let combined = format!("{}{}", stdout, stderr).to_lowercase();
        assert!(
            combined.contains("sched"),
            "Filtered output should contain sched. Got: {}",
            combined
        );
    }
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_stat_tracepoint_system_wide() {
    cleanup_test_files();

    if !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: sched:sched_switch tracepoint not found");
        return;
    }

    // System-wide tracepoint monitoring
    let (success, stdout, stderr) = run_perf(&["stat", "-a", "-e", "sched:sched_switch"]);

    // This may fail if we don't have system-wide permission
    if has_perf_permission() {
        assert!(success, "Command failed: {}", stderr);
        assert!(
            stdout.contains("sched:sched_switch") || stdout.contains("sched_switch"),
            "Output should contain tracepoint name. Got: {}",
            stdout
        );
    }
}

#[test]
#[ignore = "Requires root privileges for tracefs access"]
fn test_record_tracepoint_system_wide() {
    cleanup_test_files();

    if !tracepoint_exists("sched", "sched_switch") {
        eprintln!("Skipping test: sched:sched_switch tracepoint not found");
        return;
    }

    // System-wide tracepoint recording (short duration)
    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "-a",
        "-e",
        "sched:sched_switch",
        "-o",
        "test_tracepoint.data",
    ]);

    // This may fail if we don't have system-wide permission
    if has_perf_permission() {
        assert!(success, "Command failed: {}", stderr);
        assert!(
            Path::new("test_tracepoint.data").exists(),
            "Output file should be created"
        );
    }

    cleanup_test_files();
}

// ============================================================================
// Cleanup
// ============================================================================

#[test]
fn test_cleanup_artifacts() {
    // This test ensures cleanup runs even if other tests fail
    cleanup_test_files();
    assert!(true);
}
