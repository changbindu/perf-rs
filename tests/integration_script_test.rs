//! Integration tests for the `perf script` command.

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
fn test_script_no_input_file() {
    let (success, _stdout, stderr) = run_perf(&["script", "--input", "nonexistent_file.data"]);

    assert!(!success);
    assert!(stderr.contains("Input file not found") || stderr.contains("error"));
}

#[test]
fn test_script_help() {
    let result = Command::new("cargo")
        .args(["run", "--", "script", "--help"])
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute perf-rs");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(result.status.success());
    assert!(stdout.contains("script"));
    assert!(stdout.contains("--input") || stdout.contains("--callchain"));
}

#[test]
fn test_script_with_recorded_data() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_script.data");
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

    let (script_success, script_stdout, script_stderr) =
        run_perf(&["script", "--input", output_arg]);

    assert!(script_success, "Script failed: {}", script_stderr);
    assert!(!script_stdout.is_empty() || script_stderr.contains("No samples"));
}

#[test]
fn test_script_with_callchain() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_callchain.data");
    let output_arg = output_path.to_str().unwrap();

    let (record_success, _stdout, record_stderr) = run_perf(&[
        "record",
        "--output",
        output_arg,
        "--frequency",
        "99",
        "--call-graph",
        "--",
        "sleep",
        "0.1",
    ]);

    assert!(record_success, "Record failed: {}", record_stderr);

    let (script_success, _script_stdout, script_stderr) =
        run_perf(&["script", "--input", output_arg, "--callchain"]);

    assert!(script_success, "Script failed: {}", script_stderr);
}

#[test]
fn test_script_with_format_option() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("test_script_format.data");
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

    let (script_success, script_stdout, script_stderr) =
        run_perf(&["script", "--input", output_arg, "--format", "text"]);

    assert!(script_success, "Script failed: {}", script_stderr);
    assert!(!script_stdout.is_empty() || script_stderr.contains("No samples"));
}

#[test]
fn test_script_empty_data_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let empty_file = temp_dir.path().join("empty.data");
    std::fs::write(&empty_file, "").expect("Failed to create empty file");
    let empty_arg = empty_file.to_str().unwrap();

    let (success, _stdout, stderr) = run_perf(&["script", "--input", empty_arg]);

    assert!(!success);
    assert!(stderr.contains("Failed to open") || stderr.contains("error"));
}
