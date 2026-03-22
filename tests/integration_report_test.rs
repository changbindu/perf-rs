//! Integration tests for the `perf report` command.

use std::process::{Command, Stdio};

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

fn has_perf_permission() -> bool {
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
fn test_report_no_input_file() {
    let (success, _stdout, stderr) = run_perf(&["report", "--input", "nonexistent_file.data"]);

    assert!(!success);
    assert!(stderr.contains("Input file not found") || stderr.contains("error"));
}

#[test]
fn test_report_help() {
    let result = Command::new("cargo")
        .args(["run", "--", "report", "--help"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(result.status.success());
    assert!(stdout.contains("report"));
    assert!(stdout.contains("--input") || stdout.contains("--top"));
}

#[test]
fn test_report_with_recorded_data() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_report.data");
    let output_arg = output_path.to_str().unwrap();

    let (record_success, _stdout, record_stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);
    assert!(output_path.exists(), "Output file not created");

    let (report_success, report_stdout, report_stderr) =
        run_perf(&["report", "--input", output_arg]);

    assert!(report_success, "Report failed: {}", report_stderr);
    assert!(report_stdout.contains("Samples:") || report_stdout.contains("# Samples:"));
}

#[test]
fn test_report_with_top_option() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_top.data");
    let output_arg = output_path.to_str().unwrap();

    let (record_success, _stdout, record_stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);

    let (report_success, report_stdout, report_stderr) =
        run_perf(&["report", "--input", output_arg, "--top", "5"]);

    assert!(report_success, "Report failed: {}", report_stderr);
    assert!(report_stdout.contains("Samples:") || report_stdout.contains("# Samples:"));
}

#[test]
fn test_report_with_format_option() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_format.data");
    let output_arg = output_path.to_str().unwrap();

    let (record_success, _stdout, record_stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);

    let (report_success, report_stdout, report_stderr) =
        run_perf(&["report", "--input", output_arg, "--format", "text"]);

    assert!(report_success, "Report failed: {}", report_stderr);
    assert!(report_stdout.contains("Samples:") || report_stdout.contains("# Samples:"));
}

#[test]
fn test_report_with_sort_option() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_sort.data");
    let output_arg = output_path.to_str().unwrap();

    let (record_success, _stdout, record_stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--frequency",
        "99",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);

    let (report_success, report_stdout, report_stderr) =
        run_perf(&["report", "--input", output_arg, "--sort", "sample"]);

    assert!(report_success, "Report failed: {}", report_stderr);
    assert!(report_stdout.contains("Samples:") || report_stdout.contains("# Samples:"));
}

#[test]
fn test_report_empty_data_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let empty_file = temp_dir.path().join("empty.data");
    std::fs::write(&empty_file, "").expect("Failed to create empty file");
    let empty_arg = empty_file.to_str().unwrap();

    let (success, _stdout, stderr) = run_perf(&["report", "--input", empty_arg]);

    assert!(!success);
    assert!(stderr.contains("Failed to open") || stderr.contains("error"));
}
