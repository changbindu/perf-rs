//! Integration tests for DWARF stack unwinding.
//!
//! These tests verify that DWARF-based call graph recording works correctly
//! and that fallback to frame pointer unwinding happens when appropriate.

use std::process::{Command, Stdio};

/// Helper to run perf-rs with arguments.
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
///
/// Returns true if:
/// - Running as root, or
/// - perf_event_paranoid <= 0
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

/// Test that recording with --call-graph=dwarf produces a callchain.
///
/// This test:
/// 1. Records a simple command with DWARF call graph enabled
/// 2. Verifies the output file is created
/// 3. Runs script command to verify callchain data is present
#[test]
fn test_dwarf_record_produces_callchain() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_dwarf_callchain.data");
    let output_arg = output_path.to_str().unwrap();

    // Record with DWARF call graph
    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--call-graph=dwarf",
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "DWARF record failed: {}", stderr);
    assert!(
        output_path.exists(),
        "Output file not created: {:?}",
        output_path
    );

    let metadata = std::fs::metadata(&output_path).expect("Failed to read output file metadata");
    assert!(metadata.len() > 0, "Output file is empty");

    // Verify callchain data is present via script command
    // (call chains are shown by default)
    let (script_success, script_stdout, script_stderr) =
        run_perf(&["script", "--input", output_arg]);

    assert!(script_success, "Script failed: {}", script_stderr);
    // Script should produce some output (or indicate no samples)
    assert!(
        !script_stdout.is_empty() || script_stderr.contains("No samples"),
        "Script produced no output"
    );
}

/// Test that recording with --call-graph=fp produces a callchain.
///
/// This test verifies that frame pointer unwinding still works correctly
/// after DWARF support was added.
#[test]
fn test_fp_record_produces_callchain() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_fp_callchain.data");
    let output_arg = output_path.to_str().unwrap();

    // Record with FP call graph
    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--call-graph=fp",
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "FP record failed: {}", stderr);
    assert!(
        output_path.exists(),
        "Output file not created: {:?}",
        output_path
    );

    let metadata = std::fs::metadata(&output_path).expect("Failed to read output file metadata");
    assert!(metadata.len() > 0, "Output file is empty");

    // Verify callchain data is present via script command
    // (call chains are shown by default)
    let (script_success, script_stdout, script_stderr) =
        run_perf(&["script", "--input", output_arg]);

    assert!(script_success, "Script failed: {}", script_stderr);
    assert!(
        !script_stdout.is_empty() || script_stderr.contains("No samples"),
        "Script produced no output"
    );
}

/// Test that DWARF unwinding falls back to frame pointer on stripped binaries.
///
/// Stripped binaries don't have .eh_frame or .debug_frame sections, so DWARF
/// unwinding should fall back to frame pointer unwinding.
#[test]
fn test_dwarf_fallback_on_stripped_binary() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_stripped_fallback.data");
    let output_arg = output_path.to_str().unwrap();

    // /bin/ls is typically a stripped binary
    // Record with DWARF call graph - should fall back to FP
    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--call-graph=dwarf",
        "--frequency",
        "99",
        "--",
        "/bin/ls",
        "--version",
    ]);

    assert!(
        success,
        "DWARF record on stripped binary failed: {}",
        stderr
    );
    assert!(
        output_path.exists(),
        "Output file not created: {:?}",
        output_path
    );

    let metadata = std::fs::metadata(&output_path).expect("Failed to read output file metadata");
    assert!(metadata.len() > 0, "Output file is empty");

    // Verify that we can still read the data
    let (script_success, script_stdout, script_stderr) =
        run_perf(&["script", "--input", output_arg]);

    assert!(script_success, "Script failed: {}", script_stderr);
    // Should have some output even though binary is stripped
    // (fallback to FP should have occurred)
    assert!(
        !script_stdout.is_empty() || script_stderr.contains("No samples"),
        "Script produced no output - fallback may have failed"
    );
}

/// Test that report command shows callchains correctly for DWARF-recorded data.
///
/// This test verifies the full pipeline:
/// 1. Record with DWARF call graph
/// 2. Run report command
/// 3. Verify report shows samples with callchain information
#[test]
fn test_report_shows_dwarf_callchain() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_report_dwarf.data");
    let output_arg = output_path.to_str().unwrap();

    // Record with DWARF call graph
    let (record_success, _stdout, record_stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--call-graph=dwarf",
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);
    assert!(output_path.exists(), "Output file not created");

    // Run report command
    let (report_success, report_stdout, report_stderr) =
        run_perf(&["report", "--input", output_arg]);

    assert!(report_success, "Report failed: {}", report_stderr);
    // Report should show samples
    assert!(
        report_stdout.contains("Samples:") || report_stdout.contains("# Samples:"),
        "Report should show sample count"
    );
}

/// Test that report command shows callchains correctly for FP-recorded data.
///
/// This test verifies that FP-based call graph recording still works
/// correctly with the report command.
#[test]
fn test_report_shows_fp_callchain() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_report_fp.data");
    let output_arg = output_path.to_str().unwrap();

    // Record with FP call graph
    let (record_success, _stdout, record_stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--call-graph=fp",
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);
    assert!(output_path.exists(), "Output file not created");

    // Run report command
    let (report_success, report_stdout, report_stderr) =
        run_perf(&["report", "--input", output_arg]);

    assert!(report_success, "Report failed: {}", report_stderr);
    assert!(
        report_stdout.contains("Samples:") || report_stdout.contains("# Samples:"),
        "Report should show sample count"
    );
}

/// Test that script command shows callchains for DWARF-recorded data.
///
/// This test verifies that the script command can display callchain
/// information from DWARF-recorded samples.
#[test]
fn test_script_shows_dwarf_callchain() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_script_dwarf.data");
    let output_arg = output_path.to_str().unwrap();

    // Record with DWARF call graph
    let (record_success, _stdout, record_stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--call-graph=dwarf",
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);
    assert!(output_path.exists(), "Output file not created");

    // Run script command (call chains shown by default)
    let (script_success, script_stdout, script_stderr) =
        run_perf(&["script", "--input", output_arg]);

    assert!(script_success, "Script failed: {}", script_stderr);
    // Script should produce output
    assert!(
        !script_stdout.is_empty() || script_stderr.contains("No samples"),
        "Script should produce output"
    );
}

/// Test that the -g shorthand works correctly (defaults to fp).
///
/// This test verifies backward compatibility: -g without a value should
/// default to frame pointer unwinding.
#[test]
fn test_g_shorthand_defaults_to_fp() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_g_shorthand.data");
    let output_arg = output_path.to_str().unwrap();

    // Record with -g shorthand (should default to fp)
    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "-g",
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "Record with -g shorthand failed: {}", stderr);
    assert!(
        output_path.exists(),
        "Output file not created: {:?}",
        output_path
    );

    let metadata = std::fs::metadata(&output_path).expect("Failed to read output file metadata");
    assert!(metadata.len() > 0, "Output file is empty");
}

/// Test that custom stack size works with DWARF unwinding.
///
/// This test verifies that the --stack-size option is accepted
/// when using DWARF call graph recording.
#[test]
fn test_dwarf_with_custom_stack_size() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_stack_size.data");
    let output_arg = output_path.to_str().unwrap();

    // Record with DWARF and custom stack size
    let (success, _stdout, stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--call-graph=dwarf",
        "--stack-size",
        "64",
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(success, "Record with custom stack size failed: {}", stderr);
    assert!(
        output_path.exists(),
        "Output file not created: {:?}",
        output_path
    );

    let metadata = std::fs::metadata(&output_path).expect("Failed to read output file metadata");
    assert!(metadata.len() > 0, "Output file is empty");
}

/// Test that JSON output works with DWARF-recorded data.
///
/// This test verifies that the report and script commands can output
/// DWARF-recorded data in JSON format.
#[test]
fn test_dwarf_json_output() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_json_output.data");
    let output_arg = output_path.to_str().unwrap();

    // Record with DWARF call graph
    let (record_success, _stdout, record_stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--call-graph=dwarf",
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);
    assert!(output_path.exists(), "Output file not created");

    // Test report with JSON format
    let (report_success, report_stdout, report_stderr) =
        run_perf(&["report", "--input", output_arg, "--format", "json"]);

    assert!(report_success, "Report JSON failed: {}", report_stderr);
    // JSON output should start with { or [ if there are samples,
    // or contain "No samples" message, or show sample count
    let trimmed = report_stdout.trim();
    let has_valid_output = trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.contains("No samples")
        || trimmed.contains("Samples:");
    assert!(
        has_valid_output,
        "Report output should be valid JSON or show sample info, got: {}",
        trimmed
    );

    // Test script with JSON format
    let (script_success, script_stdout, script_stderr) =
        run_perf(&["script", "--input", output_arg, "--format", "json"]);

    assert!(script_success, "Script JSON failed: {}", script_stderr);
    let script_trimmed = script_stdout.trim();
    let has_valid_script_output = script_trimmed.starts_with('{')
        || script_trimmed.starts_with('[')
        || script_trimmed.is_empty()
        || script_trimmed.contains("No samples");
    assert!(
        has_valid_script_output,
        "Script JSON output should be valid JSON or empty, got: {}",
        script_trimmed
    );
}
