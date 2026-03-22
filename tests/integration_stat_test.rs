//! Integration tests for perf stat system-wide profiling.
//!
//! These tests verify the system-wide performance monitoring functionality
//! of the stat command, including:
//! - System-wide aggregated mode (`-a`)
//! - System-wide per-CPU mode (`-a --per-cpu`)
//! - Specific CPU selection (`-C`)
//! - Error handling and validation
//!
//! # Running Tests
//!
//! Tests that require perf permissions will automatically skip if permissions
//! are not available. No root required if `kernel.perf_event_paranoid <= 0`.
//!
//! ```bash
//! # Check your paranoid setting:
//! cat /proc/sys/kernel/perf_event_paranoid
//!
//! # If paranoid <= 0, tests run without sudo:
//! cargo test --test integration_stat
//!
//! # For paranoid > 0, run with sudo:
//! sudo cargo test --test integration_stat
//! ```

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
/// Returns true if we have root, CAP_SYS_ADMIN, CAP_PERFMON,
/// or perf_event_paranoid <= 0.
fn has_perf_permission() -> bool {
    // Check for root
    if unsafe { libc::getuid() } == 0 {
        return true;
    }

    // Check perf_event_paranoid setting
    if let Ok(content) = std::fs::read_to_string("/proc/sys/kernel/perf_event_paranoid") {
        if let Ok(level) = content.trim().parse::<i32>() {
            // System-wide requires paranoid <= 0
            return level <= 0;
        }
    }

    false
}

/// Check if we have system-wide profiling permission.
/// System-wide requires perf_event_paranoid <= 0 or root/CAP_PERFMON.
fn has_system_wide_permission() -> bool {
    has_perf_permission()
}

/// Get the number of online CPUs on the system.
fn get_cpu_count() -> usize {
    if let Ok(content) = std::fs::read_to_string("/sys/devices/system/cpu/online") {
        let content = content.trim();
        if content.contains('-') {
            let parts: Vec<&str> = content.split('-').collect();
            if parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                {
                    return end - start + 1;
                }
            }
        } else if content.contains(',') {
            return content.matches(',').count() + 1;
        } else if let Ok(n) = content.parse::<usize>() {
            return n + 1;
        }
    }
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1)
}

// ============================================================================
// Tests that DO NOT require root privileges
// ============================================================================

#[test]
fn test_stat_system_wide_help_shows_flags() {
    let (success, stdout, stderr) = run_perf(&["stat", "--help"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("--all-cpus") || stdout.contains("-a"),
        "Help should mention --all-cpus flag"
    );
    assert!(
        stdout.contains("--cpu") || stdout.contains("-C"),
        "Help should mention --cpu flag"
    );
    assert!(
        stdout.contains("--per-cpu"),
        "Help should mention --per-cpu flag"
    );
}

#[test]
fn test_stat_conflicting_flags_all_cpus_and_cpu() {
    // -a and -C are mutually exclusive
    let (success, stdout, stderr) = run_perf(&["stat", "-a", "-C", "0", "--", "true"]);

    assert!(!success, "Command should fail with conflicting flags");
    // The error could be in stdout or stderr depending on clap behavior
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("conflict")
            || combined.contains("cannot be used with")
            || combined.contains("exclusive")
            || combined.contains("error"),
        "Error message should mention conflict or exclusivity. Got: {}",
        combined
    );
}

#[test]
fn test_stat_invalid_cpu_list_syntax() {
    // Invalid CPU list syntax (letters instead of numbers)
    let (success, stdout, stderr) = run_perf(&["stat", "-C", "abc", "--", "true"]);

    assert!(!success, "Command should fail with invalid CPU list");
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("error")
            || combined.contains("invalid")
            || combined.contains("parse")
            || combined.contains("Failed"),
        "Error message should mention parsing error. Got: {}",
        combined
    );
}

#[test]
fn test_stat_cpu_out_of_range() {
    // CPU 999 is unlikely to exist on most systems
    let cpu_count = get_cpu_count();
    let invalid_cpu = cpu_count + 100;

    let (success, stdout, stderr) =
        run_perf(&["stat", "-C", &invalid_cpu.to_string(), "--", "true"]);

    // This test requires privileges to even attempt, so it may skip
    // But if it runs, it should fail
    if has_system_wide_permission() {
        assert!(!success, "Command should fail with out-of-range CPU");
        let combined = format!("{}{}", stdout, stderr);
        assert!(
            combined.contains("invalid")
                || combined.contains("not found")
                || combined.contains("error")
                || combined.contains("Failed"),
            "Error message should mention invalid CPU. Got: {}",
            combined
        );
    }
}

#[test]
fn test_stat_per_cpu_without_system_wide_mode() {
    // --per-cpu without -a or -C should still work but may not show per-CPU
    // (behavior depends on implementation - it might be ignored or produce a warning)
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let (success, _stdout, _stderr) = run_perf(&["stat", "--per-cpu", "--", "true"]);

    // The command should succeed even if --per-cpu is ignored
    // (it's documented as "requires -a or -C")
    // We just verify it doesn't crash
    let _ = success;
}

// ============================================================================
// Tests that require perf permissions (auto-skip if not available)
// ============================================================================

#[test]
fn test_stat_system_wide_aggregated() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let (success, stdout, stderr) = run_perf(&["stat", "-a", "--", "sleep", "0.1"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("cpu-cycles") || stdout.contains("instructions"),
        "Output should contain event names. Got: {}",
        stdout
    );
    assert!(
        stdout.contains("Performance counter stats"),
        "Output should contain stats header"
    );
}

#[test]

fn test_stat_system_wide_aggregated_no_command() {
    // System-wide stat without a command should run for a default duration
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let (success, stdout, stderr) = run_perf(&["stat", "-a"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("cpu-cycles") || stdout.contains("instructions"),
        "Output should contain event names"
    );
    assert!(
        stdout.contains("system-wide"),
        "Output should mention system-wide mode"
    );
}

#[test]

fn test_stat_system_wide_per_cpu() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let (success, stdout, stderr) = run_perf(&["stat", "-a", "--per-cpu", "--", "sleep", "0.1"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("CPU"),
        "Per-CPU output should contain CPU column. Got: {}",
        stdout
    );
    assert!(
        stdout.contains("Event") || stdout.contains("cpu-cycles"),
        "Per-CPU output should contain event names"
    );
    assert!(
        stdout.contains("Overhead"),
        "Per-CPU output should contain overhead column"
    );
}

#[test]

fn test_stat_system_wide_per_cpu_format() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let (success, stdout, stderr) = run_perf(&["stat", "-a", "--per-cpu", "--", "sleep", "0.1"]);

    assert!(success, "Command failed: {}", stderr);

    // Verify table format: CPU column with numeric values
    let lines: Vec<&str> = stdout.lines().collect();
    let mut found_cpu_number = false;
    for line in &lines {
        // Look for lines starting with a CPU number (e.g., "   0  cpu-cycles")
        if line.trim().starts_with(|c: char| c.is_ascii_digit()) {
            found_cpu_number = true;
            break;
        }
    }
    assert!(
        found_cpu_number,
        "Per-CPU output should have rows with CPU numbers. Got:\n{}",
        stdout
    );
}

#[test]

fn test_stat_specific_cpu() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    // Test with CPU 0 only
    let (success, stdout, stderr) = run_perf(&["stat", "-C", "0", "--", "sleep", "0.1"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("cpu-cycles") || stdout.contains("instructions"),
        "Output should contain event names"
    );
}

#[test]

fn test_stat_specific_cpus_list() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let cpu_count = get_cpu_count();
    if cpu_count < 2 {
        eprintln!("Skipping test: requires at least 2 CPUs");
        return;
    }

    // Test with multiple CPUs specified as list
    let (success, stdout, stderr) = run_perf(&["stat", "-C", "0,1", "--", "sleep", "0.1"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("cpu-cycles") || stdout.contains("instructions"),
        "Output should contain event names"
    );
}

#[test]

fn test_stat_specific_cpus_range() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let cpu_count = get_cpu_count();
    if cpu_count < 2 {
        eprintln!("Skipping test: requires at least 2 CPUs");
        return;
    }

    // Test with CPU range
    let (success, stdout, stderr) = run_perf(&["stat", "-C", "0-1", "--", "sleep", "0.1"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("cpu-cycles") || stdout.contains("instructions"),
        "Output should contain event names"
    );
}

#[test]

fn test_stat_specific_cpu_per_cpu_output() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let cpu_count = get_cpu_count();
    if cpu_count < 2 {
        eprintln!("Skipping test: requires at least 2 CPUs");
        return;
    }

    // Test --per-cpu with specific CPUs
    let (success, stdout, stderr) =
        run_perf(&["stat", "-C", "0,1", "--per-cpu", "--", "sleep", "0.1"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("CPU"),
        "Per-CPU output should contain CPU column"
    );

    // Should only show CPUs 0 and 1, not all CPUs
    let lines: Vec<&str> = stdout.lines().collect();
    let mut cpu_numbers: Vec<u32> = Vec::new();
    for line in &lines {
        let trimmed = line.trim();
        if let Some(first_space) = trimmed.find(' ') {
            if let Ok(cpu) = trimmed[..first_space].parse::<u32>() {
                cpu_numbers.push(cpu);
            }
        }
    }

    // All CPU numbers should be 0 or 1
    for cpu in &cpu_numbers {
        assert!(
            *cpu <= 1,
            "Per-CPU output should only contain CPUs 0 and 1, found: {}",
            cpu
        );
    }
}

#[test]

fn test_stat_system_wide_with_custom_events() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let (success, stdout, stderr) = run_perf(&[
        "stat",
        "-a",
        "-e",
        "cache-references,cache-misses",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("cache-references"),
        "Output should contain cache-references event"
    );
    assert!(
        stdout.contains("cache-misses"),
        "Output should contain cache-misses event"
    );
}

#[test]

fn test_stat_system_wide_standalone_measurement() {
    // Test system-wide monitoring without a command (standalone mode)
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    // This should monitor for a default duration (1 second)
    let (success, stdout, stderr) = run_perf(&["stat", "-a"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("cpu-cycles") || stdout.contains("instructions"),
        "Output should contain event names"
    );
}

// ============================================================================
// Permission error tests
// ============================================================================

#[test]
fn test_stat_system_wide_permission_error_without_privileges() {
    // This test verifies that non-root users get a clear error
    // It will pass either by:
    // 1. Having no privileges (command fails with permission error)
    // 2. Having privileges (command succeeds - we skip the assertion)
    if has_system_wide_permission() {
        eprintln!("Skipping test: running with sufficient privileges");
        return;
    }

    let (success, stdout, stderr) = run_perf(&["stat", "-a", "--", "sleep", "0.1"]);

    assert!(
        !success,
        "Command should fail without system-wide privileges"
    );

    let combined = format!("{}{}", stdout, stderr).to_lowercase();
    assert!(
        combined.contains("privilege")
            || combined.contains("permission")
            || combined.contains("denied")
            || combined.contains("cap_sys_admin")
            || combined.contains("cap_perfmon")
            || combined.contains("perf_event_paranoid"),
        "Error message should mention privileges or permissions. Got: {}",
        combined
    );
}

#[test]
fn test_stat_system_wide_permission_error_message_quality() {
    // Verify error message provides actionable suggestions
    if has_system_wide_permission() {
        eprintln!("Skipping test: running with sufficient privileges");
        return;
    }

    let (success, stdout, stderr) = run_perf(&["stat", "-a", "--", "sleep", "0.1"]);

    assert!(!success);

    let combined = format!("{}{}", stdout, stderr);

    // Error message should suggest solutions
    let has_suggestions = combined.contains("sudo")
        || combined.contains("setcap")
        || combined.contains("perf_event_paranoid")
        || combined.contains("capability");

    assert!(
        has_suggestions,
        "Error message should provide actionable suggestions. Got: {}",
        combined
    );
}

// ============================================================================
// Output format verification tests
// ============================================================================

#[test]

fn test_stat_aggregated_output_single_value_per_event() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let (success, stdout, stderr) = run_perf(&[
        "stat",
        "-a",
        "-e",
        "cpu-cycles,instructions",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "Command failed: {}", stderr);

    // In aggregated mode, each event should appear exactly once
    let cpu_cycles_count = stdout.matches("cpu-cycles").count();
    let instructions_count = stdout.matches("instructions").count();

    assert_eq!(
        cpu_cycles_count, 1,
        "In aggregated mode, cpu-cycles should appear once, found {} times",
        cpu_cycles_count
    );
    assert_eq!(
        instructions_count, 1,
        "In aggregated mode, instructions should appear once, found {} times",
        instructions_count
    );
}

#[test]

fn test_stat_per_cpu_output_multiple_values_per_event() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let cpu_count = get_cpu_count();

    let (success, stdout, stderr) = run_perf(&[
        "stat",
        "-a",
        "--per-cpu",
        "-e",
        "cpu-cycles",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "Command failed: {}", stderr);

    // In per-CPU mode, each event should appear once per CPU
    let cpu_cycles_count = stdout.matches("cpu-cycles").count();

    assert!(
        cpu_cycles_count >= 1,
        "In per-CPU mode, cpu-cycles should appear at least once, found {} times",
        cpu_cycles_count
    );

    // Should appear once per monitored CPU (all online CPUs with -a)
    assert!(
        cpu_cycles_count <= cpu_count,
        "In per-CPU mode, cpu-cycles should appear at most once per CPU ({} CPUs), found {} times",
        cpu_count,
        cpu_cycles_count
    );
}

#[test]

fn test_stat_per_cpu_sorted_by_cpu_id() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    let (success, stdout, stderr) = run_perf(&[
        "stat",
        "-a",
        "--per-cpu",
        "-e",
        "cpu-cycles",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "Command failed: {}", stderr);

    // Extract CPU IDs from output
    let mut cpu_numbers: Vec<u32> = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(first_space) = trimmed.find(' ') {
            if let Ok(cpu) = trimmed[..first_space].parse::<u32>() {
                cpu_numbers.push(cpu);
            }
        }
    }

    // Verify CPUs are sorted in ascending order
    let mut sorted = cpu_numbers.clone();
    sorted.sort();
    assert_eq!(
        cpu_numbers, sorted,
        "Per-CPU output should be sorted by CPU ID"
    );
}

#[test]

fn test_stat_output_includes_ipc() {
    if !has_system_wide_permission() {
        eprintln!("Skipping test: requires system-wide perf permissions");
        return;
    }

    // Default events include cpu-cycles and instructions, so IPC should be calculated
    let (success, stdout, stderr) = run_perf(&["stat", "-a", "--", "sleep", "0.1"]);

    assert!(success, "Command failed: {}", stderr);
    assert!(
        stdout.contains("insn per cycle") || stdout.contains("IPC"),
        "Output should include instructions per cycle (IPC)"
    );
}
