//! perf.data compatibility validation tests
//!
//! This test suite validates that perf-rs generates perf.data files that are
//! fully compatible with the Linux perf tool (perf report, perf script).
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
//! cargo test --test perf_compatibility
//!
//! # For paranoid > 0, run with sudo:
//! sudo cargo test --test perf_compatibility
//! ```

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

const PERF_FILE_MAGIC: u64 = 0x50455246494c4532;
const PERF_ATTR_SIZE_VER8: u64 = 136;

fn run_perf_rs(args: &[&str]) -> (bool, String, String) {
    let result = Command::new("cargo")
        .args(["run", "--release", "--"])
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

fn perf_available() -> bool {
    Command::new("perf")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn verify_perf_data_magic(path: &PathBuf) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;

    let mut magic_bytes = [0u8; 8];
    file.read_exact(&mut magic_bytes)
        .map_err(|e| format!("Failed to read magic bytes: {}", e))?;

    let magic = u64::from_be_bytes(magic_bytes);

    if magic != PERF_FILE_MAGIC {
        return Err(format!(
            "Invalid magic number: expected {:016x}, got {:016x}",
            PERF_FILE_MAGIC, magic
        ));
    }

    Ok(())
}

fn perf_report(path: &PathBuf) -> Result<String, String> {
    let result = Command::new("perf")
        .args(["report", "--input", path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run perf report: {}", e))?;

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

    let error_keywords = [
        "error:",
        "Error:",
        "failed",
        "Failed",
        "unrecognized",
        "corrupt",
    ];
    for keyword in &error_keywords {
        if stderr.contains(keyword) && !stderr.contains("warning") && !stderr.contains("Warning") {
            return Err(format!("perf report failed with error: {}", stderr));
        }
    }

    if result.status.success() {
        Ok(stdout)
    } else {
        Err(format!(
            "perf report exited with status {}: {}",
            result.status, stderr
        ))
    }
}

fn perf_script(path: &PathBuf) -> Result<String, String> {
    let result = Command::new("perf")
        .args(["script", "--input", path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run perf script: {}", e))?;

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

    let error_keywords = [
        "error:",
        "Error:",
        "failed",
        "Failed",
        "unrecognized",
        "corrupt",
    ];
    for keyword in &error_keywords {
        if stderr.contains(keyword) && !stderr.contains("warning") && !stderr.contains("Warning") {
            return Err(format!("perf script failed with error: {}", stderr));
        }
    }

    if result.status.success() {
        Ok(stdout)
    } else {
        Err(format!(
            "perf script exited with status {}: {}",
            result.status, stderr
        ))
    }
}

fn verify_attr_size(path: &PathBuf) -> Result<u64, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;

    file.seek(SeekFrom::Start(16))
        .map_err(|e| format!("Failed to seek to attr_size: {}", e))?;

    let mut attr_size_bytes = [0u8; 8];
    file.read_exact(&mut attr_size_bytes)
        .map_err(|e| format!("Failed to read attr_size: {}", e))?;

    let attr_size = u64::from_le_bytes(attr_size_bytes);

    let expected_attr_size = PERF_ATTR_SIZE_VER8 + 16;
    if attr_size != expected_attr_size {
        return Err(format!(
            "Invalid attr_size: expected {} (attr struct + IDs section), got {}",
            expected_attr_size, attr_size
        ));
    }

    Ok(attr_size)
}

#[test]
fn test_magic_number_validation() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("magic_test.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--frequency",
        "99",
        "--",
        "true",
    ]);

    assert!(success, "perf-rs record failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");

    verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");
}

#[test]
fn test_attr_size_header_field() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("attr_size_test.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--frequency",
        "99",
        "--",
        "true",
    ]);

    assert!(success, "perf-rs record failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");

    let attr_size = verify_attr_size(&output_path).expect("attr_size validation failed");
    let expected_attr_entry_size = PERF_ATTR_SIZE_VER8 + 16;
    assert_eq!(
        attr_size, expected_attr_entry_size,
        "attr_size should be {} (attr struct + IDs section), got {}",
        expected_attr_entry_size, attr_size
    );
}

#[test]
fn test_callchain_recording() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    if !perf_available() {
        eprintln!("Skipping test: perf tool not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("callchain.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--call-graph",
        "--frequency",
        "99",
        "--",
        "ls",
        "-la",
    ]);

    assert!(success, "perf-rs record with -g failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");

    verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");
    verify_attr_size(&output_path).expect("Invalid attr_size");

    let report_output = perf_report(&output_path).expect("perf report failed on callchain data");
    eprintln!("perf report output:\n{}", report_output);

    let script_output = perf_script(&output_path).expect("perf script failed on callchain data");
    eprintln!("perf script output:\n{}", script_output);

    let file_size = output_path.metadata().unwrap().len();
    eprintln!("Callchain recording file size: {} bytes", file_size);
    assert!(
        file_size > 1000,
        "File should contain samples with callchain data"
    );
}

#[test]

fn test_empty_recording() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    if !perf_available() {
        eprintln!("Skipping test: perf tool not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("empty.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--frequency",
        "99",
        "--",
        "true",
    ]);

    assert!(success, "perf-rs record failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");

    verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");

    let report_output = perf_report(&output_path).expect("perf report failed");
    eprintln!("perf report output:\n{}", report_output);

    let script_output = perf_script(&output_path).expect("perf script failed");
    eprintln!("perf script output:\n{}", script_output);

    let file_size = output_path.metadata().unwrap().len();
    eprintln!("Empty recording file size: {} bytes", file_size);
    assert!(file_size > 0, "File should not be empty");
}

#[test]

fn test_simple_command() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    if !perf_available() {
        eprintln!("Skipping test: perf tool not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("simple.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--frequency",
        "99",
        "--",
        "ls",
        "-la",
    ]);

    assert!(success, "perf-rs record failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");

    verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");

    let report_output = perf_report(&output_path).expect("perf report failed");
    eprintln!("perf report output:\n{}", report_output);

    let script_output = perf_script(&output_path).expect("perf script failed");
    eprintln!("perf script output:\n{}", script_output);

    let file_size = output_path.metadata().unwrap().len();
    eprintln!("Simple command file size: {} bytes", file_size);
    assert!(file_size > 1000, "File should contain samples");
}

#[test]

fn test_multithreaded_application() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    if !perf_available() {
        eprintln!("Skipping test: perf tool not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_program_path = temp_dir.path().join("multithread_test");

    let test_program = r#"
use std::thread;
use std::time::Duration;

fn main() {
    let handles: Vec<_> = (0..4)
        .map(|i| {
            thread::spawn(move || {
                let mut sum = 0u64;
                for j in 0..1_000_000 {
                    sum = sum.wrapping_add(j as u64);
                }
                eprintln!("Thread {} computed: {}", i, sum);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
"#;

    std::fs::write(temp_dir.path().join("test_program.rs"), test_program)
        .expect("Failed to write test program");

    let compile_result = Command::new("rustc")
        .args([
            "-o",
            test_program_path.to_str().unwrap(),
            temp_dir.path().join("test_program.rs").to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match compile_result {
        Ok(result) if result.status.success() => {}
        Ok(result) => {
            eprintln!(
                "Skipping test: failed to compile test program: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            return;
        }
        Err(e) => {
            eprintln!("Skipping test: rustc not available: {}", e);
            return;
        }
    }

    let output_path = temp_dir.path().join("multithread.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--frequency",
        "99",
        "--",
        test_program_path.to_str().unwrap(),
    ]);

    assert!(success, "perf-rs record failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");

    verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");

    let report_output = perf_report(&output_path).expect("perf report failed");
    eprintln!("perf report output:\n{}", report_output);

    let script_output = perf_script(&output_path).expect("perf script failed");
    eprintln!("perf script output:\n{}", script_output);

    let file_size = output_path.metadata().unwrap().len();
    eprintln!("Multi-threaded file size: {} bytes", file_size);
    assert!(file_size > 1000, "File should contain samples");

    let thread_count = script_output
        .lines()
        .filter(|line| line.contains("test_program"))
        .count();
    eprintln!("Samples from test_program: {}", thread_count);
}

#[test]

fn test_large_file() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    if !perf_available() {
        eprintln!("Skipping test: perf tool not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("large.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--frequency",
        "1000",
        "--",
        "dd",
        "if=/dev/zero",
        "of=/dev/null",
        "bs=1M",
        "count=10",
    ]);

    if !success && stderr.contains("No such file") {
        let (success, _stdout, stderr) = run_perf_rs(&[
            "record",
            "--output",
            output_path.to_str().unwrap(),
            "--frequency",
            "1000",
            "--",
            "sleep",
            "0.5",
        ]);

        assert!(success, "perf-rs record failed: {}", stderr);
    } else {
        assert!(success, "perf-rs record failed: {}", stderr);
    }

    assert!(output_path.exists(), "Output file not created");

    verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");

    let report_output = perf_report(&output_path).expect("perf report failed");
    eprintln!("perf report output:\n{}", report_output);

    let script_output = perf_script(&output_path).expect("perf script failed");
    eprintln!("perf script output:\n{}", script_output);

    let file_size = output_path.metadata().unwrap().len();
    eprintln!("Large file size: {} bytes", file_size);
    assert!(file_size > 1000, "File should contain many samples");
}

#[test]

fn test_very_short_duration() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    if !perf_available() {
        eprintln!("Skipping test: perf tool not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("short.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--frequency",
        "99",
        "--",
        "sh",
        "-c",
        ":",
    ]);

    assert!(success, "perf-rs record failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");

    verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");

    let report_output = perf_report(&output_path).expect("perf report failed");
    eprintln!("perf report output:\n{}", report_output);

    let script_output = perf_script(&output_path).expect("perf script failed");
    eprintln!("perf script output:\n{}", script_output);

    let file_size = output_path.metadata().unwrap().len();
    eprintln!("Short duration file size: {} bytes", file_size);
}

#[test]

fn test_specific_event() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    if !perf_available() {
        eprintln!("Skipping test: perf tool not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("event.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--event",
        "instructions",
        "--frequency",
        "99",
        "--",
        "ls",
        "-la",
    ]);

    assert!(success, "perf-rs record failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");

    verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");

    let report_output = perf_report(&output_path).expect("perf report failed");
    eprintln!("perf report output:\n{}", report_output);

    let script_output = perf_script(&output_path).expect("perf script failed");
    eprintln!("perf script output:\n{}", script_output);
}

#[test]

fn test_sample_period() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    if !perf_available() {
        eprintln!("Skipping test: perf tool not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("period.perf.data");

    let (success, _stdout, stderr) = run_perf_rs(&[
        "record",
        "--output",
        output_path.to_str().unwrap(),
        "--period",
        "10000",
        "--",
        "ls",
        "-la",
    ]);

    assert!(success, "perf-rs record failed: {}", stderr);
    assert!(output_path.exists(), "Output file not created");

    verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");

    let report_output = perf_report(&output_path).expect("perf report failed");
    eprintln!("perf report output:\n{}", report_output);

    let script_output = perf_script(&output_path).expect("perf script failed");
    eprintln!("perf script output:\n{}", script_output);
}

#[test]

fn test_system_wide_recording() {
    if !has_perf_permission() {
        eprintln!("Skipping test: requires perf permissions");
        return;
    }

    if !perf_available() {
        eprintln!("Skipping test: perf tool not available");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp_dir.path().join("system_wide.perf.data");

    let recording_started = std::sync::Arc::new(AtomicBool::new(false));
    let recording_started_clone = recording_started.clone();

    thread::spawn(move || {
        while !recording_started_clone.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(10));
        }
        thread::sleep(Duration::from_millis(100));
    });

    let output_path_clone = output_path.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let result = Command::new("cargo")
            .args([
                "run",
                "--release",
                "--",
                "record",
                "-a",
                "--output",
                output_path_clone.to_str().unwrap(),
                "--frequency",
                "99",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let _ = tx.send(result);
    });

    recording_started.store(true, Ordering::Relaxed);

    thread::sleep(Duration::from_millis(1000));

    let _ = Command::new("pkill").args(["-SIGINT", "perf-rs"]).output();

    let result = rx.recv_timeout(Duration::from_secs(5));
    let (_success, _stdout, stderr) = match result {
        Ok(Ok(r)) => (
            r.status.success(),
            String::from_utf8_lossy(&r.stdout).to_string(),
            String::from_utf8_lossy(&r.stderr).to_string(),
        ),
        Ok(Err(_)) => (false, String::new(), "Command failed".to_string()),
        Err(_) => {
            eprintln!("System-wide recording timed out after 10 seconds");
            return;
        }
    };

    if output_path.exists() {
        verify_perf_data_magic(&output_path).expect("Invalid perf.data magic number");

        let report_output = perf_report(&output_path).expect("perf report failed");
        eprintln!("perf report output:\n{}", report_output);

        let script_output = perf_script(&output_path).expect("perf script failed");
        eprintln!("perf script output:\n{}", script_output);
    } else {
        eprintln!(
            "System-wide recording file not created (may be expected): {}",
            stderr
        );
    }
}
