//! Performance recording command - collects samples for profiling.

use crate::core::cpu::{get_online_cpus, parse_cpu_list, validate_cpu_ids};
use crate::core::perf_data::{PerfDataWriter, SampleEvent};
use crate::core::perf_event::Hardware;
use crate::core::privilege::check_privilege;
use crate::error::PerfError;
use anyhow::{Context, Result};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{execvp, fork, ForkResult, Pid};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn signal_handler(_sig: libc::c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

fn parse_event(name: &str) -> Result<Hardware> {
    match name.trim().to_lowercase().as_str() {
        "cpu-cycles" | "cycles" => Ok(Hardware::CPU_CYCLES),
        "instructions" | "instructions-retired" => Ok(Hardware::INSTRUCTIONS),
        "cache-references" => Ok(Hardware::CACHE_REFERENCES),
        "cache-misses" => Ok(Hardware::CACHE_MISSES),
        "branch-instructions" | "branches" => Ok(Hardware::BRANCH_INSTRUCTIONS),
        "branch-misses" => Ok(Hardware::BRANCH_MISSES),
        "stalled-cycles-frontend" | "idle-cycles-frontend" => Ok(Hardware::STALLED_CYCLES_FRONTEND),
        "stalled-cycles-backend" | "idle-cycles-backend" => Ok(Hardware::STALLED_CYCLES_BACKEND),
        "ref-cpu-cycles" | "cpu-cycles-ref" => Ok(Hardware::REF_CPU_CYCLES),
        _ => Err(anyhow::anyhow!(
            "Unknown event: '{}'. Supported events: cpu-cycles, instructions, cache-references, cache-misses, branch-instructions, branch-misses",
            name
        )),
    }
}

fn setup_signal_handlers() -> Result<()> {
    let sig_action = nix::sys::signal::SigHandler::Handler(signal_handler);
    let flags = nix::sys::signal::SaFlags::empty();
    let mask = nix::sys::signal::SigSet::empty();
    let action = nix::sys::signal::SigAction::new(sig_action, flags, mask);

    unsafe {
        nix::sys::signal::sigaction(Signal::SIGINT, &action)
            .context("Failed to register SIGINT handler")?;
        nix::sys::signal::sigaction(Signal::SIGTERM, &action)
            .context("Failed to register SIGTERM handler")?;
    }

    Ok(())
}

pub fn execute(
    pid: Option<u32>,
    all_cpus: bool,
    cpu: Option<&str>,
    output: Option<&str>,
    event: Option<&str>,
    frequency: Option<u64>,
    period: Option<u64>,
    command: &[String],
) -> Result<()> {
    let privilege_level = check_privilege().map_err(|e| PerfError::PermissionDenied {
        operation: e.to_string(),
    })?;

    let is_system_wide = all_cpus || cpu.is_some();

    if is_system_wide {
        if !privilege_level.can_profile_system_wide() {
            eprintln!("Error: Insufficient privileges for system-wide profiling.");
            for suggestion in privilege_level.suggestions() {
                eprintln!("  {}", suggestion);
            }
            std::process::exit(1);
        }
    } else if !privilege_level.can_profile() {
        eprintln!("Error: Insufficient privileges for performance monitoring.");
        for suggestion in privilege_level.suggestions() {
            eprintln!("  {}", suggestion);
        }
        std::process::exit(1);
    }

    let event = if let Some(event_str) = event {
        parse_event(event_str)?
    } else {
        Hardware::CPU_CYCLES
    };

    let sample_period = if let Some(freq) = frequency {
        1_000_000_000u64 / freq.max(1)
    } else if let Some(p) = period {
        p
    } else {
        1_000_000
    };

    let output_path = output.unwrap_or("perf.data");

    setup_signal_handlers()?;

    if is_system_wide {
        let cpus = if all_cpus {
            get_online_cpus().context("Failed to get online CPUs")?
        } else {
            let cpu_str = cpu.unwrap();
            let cpus = parse_cpu_list(cpu_str).context("Failed to parse CPU list")?;
            let online_cpus = get_online_cpus().context("Failed to get online CPUs")?;
            let max_cpu = online_cpus.iter().max().copied().unwrap_or(0);
            validate_cpu_ids(&cpus, max_cpu).context("Invalid CPU ID in list")?;
            cpus
        };

        record_system_wide(cpus, event, sample_period, output_path)
    } else if let Some(target_pid) = pid {
        record_with_pid(target_pid, event, sample_period, output_path)
    } else if command.is_empty() {
        Err(anyhow::anyhow!(
            "No command specified. Usage: perf record -- <command> [args...]"
        ))
    } else {
        record_with_command(command, event, sample_period, output_path)
    }
}

fn record_with_pid(pid: u32, event: Hardware, sample_period: u64, output_path: &str) -> Result<()> {
    eprintln!("Recording process {} ...", pid);

    let mut ringbuf = crate::core::ringbuf::RingBuffer::from_event_for_pid(
        event,
        pid as i32,
        sample_period,
        false,
    )
    .context("Failed to create ring buffer")?;

    ringbuf.enable().context("Failed to enable sampling")?;

    let mut writer = PerfDataWriter::from_path(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path))?;

    let start_time = Instant::now();
    let mut sample_count = 0u64;

    while !SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
        if !process_exists(pid) {
            break;
        }

        while let Some(record) = ringbuf.next_record() {
            if let Some(sample) = parse_sample_record(&record, sample_period) {
                writer
                    .write_sample(&sample)
                    .context("Failed to write sample")?;
                sample_count += 1;
            }
        }

        std::thread::sleep(Duration::from_micros(100));
    }

    ringbuf.disable().ok();

    writer
        .finalize_with_header_update()
        .context("Failed to finalize output file")?;

    let elapsed = start_time.elapsed();
    eprintln!(
        "Recorded {} samples in {:.2}s ({} lost)",
        sample_count,
        elapsed.as_secs_f64(),
        ringbuf.lost_count()
    );
    eprintln!("Saved to: {}", output_path);

    Ok(())
}

fn record_with_command(
    command: &[String],
    event: Hardware,
    sample_period: u64,
    output_path: &str,
) -> Result<()> {
    match unsafe { fork() }? {
        ForkResult::Parent { child } => {
            waitpid(child, Some(WaitPidFlag::WUNTRACED))
                .context("Failed to wait for child to stop")?;

            let mut ringbuf = crate::core::ringbuf::RingBuffer::from_event_for_pid(
                event,
                child.as_raw(),
                sample_period,
                false,
            )
            .context("Failed to create ring buffer")?;

            let mut writer = PerfDataWriter::from_path(output_path)
                .with_context(|| format!("Failed to create output file: {}", output_path))?;

            ringbuf.enable().context("Failed to enable sampling")?;

            kill(child, Signal::SIGCONT).context("Failed to continue child process")?;

            let start_time = Instant::now();
            let mut sample_count = 0u64;

            loop {
                if SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
                    kill(child, Signal::SIGKILL).ok();
                    break;
                }

                match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                    Ok(WaitStatus::Exited(_, _)) | Ok(WaitStatus::Signaled(_, _, _)) => break,
                    Ok(WaitStatus::StillAlive) => {}
                    Err(_) => break,
                    _ => {}
                }

                while let Some(record) = ringbuf.next_record() {
                    if let Some(sample) = parse_sample_record(&record, sample_period) {
                        writer
                            .write_sample(&sample)
                            .context("Failed to write sample")?;
                        sample_count += 1;
                    }
                }

                std::thread::sleep(Duration::from_micros(100));
            }

            let status = waitpid(child, None).ok();

            ringbuf.disable().ok();

            writer
                .finalize_with_header_update()
                .context("Failed to finalize output file")?;

            let elapsed = start_time.elapsed();
            eprintln!(
                "Recorded {} samples in {:.2}s ({} lost)",
                sample_count,
                elapsed.as_secs_f64(),
                ringbuf.lost_count()
            );
            eprintln!("Saved to: {}", output_path);

            if let Some(WaitStatus::Exited(_, code)) = status {
                if code != 0 {
                    eprintln!("  Process exited with status: {}", code);
                }
            }

            Ok(())
        }
        ForkResult::Child => {
            kill(Pid::this(), Signal::SIGSTOP).ok();

            let program = &command[0];
            let args: Vec<std::ffi::CString> = command
                .iter()
                .map(|s| {
                    std::ffi::CString::new(s.as_bytes())
                        .expect("Command argument contains null byte")
                })
                .collect();

            execvp(
                std::ffi::CString::new(program.as_bytes())
                    .expect("Program name contains null byte")
                    .as_c_str(),
                &args.iter().map(|s| s.as_c_str()).collect::<Vec<_>>(),
            )
            .context("Failed to execute command")?;

            unreachable!()
        }
    }
}

fn process_exists(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{}", pid)).exists()
}

fn parse_sample_record(record: &perf_event::Record<'_>, sample_period: u64) -> Option<SampleEvent> {
    use perf_event::data::Record as DataRecord;

    let parsed = record.parse_record().ok()?;

    match parsed {
        DataRecord::Sample(sample) => {
            let ip = sample.ip()?;
            let pid = sample.pid()?;
            let tid = sample.tid()?;
            let time = sample.time().unwrap_or(0);
            let cpu = sample.cpu();

            let callchain = sample.callchain().map(|c| c.to_vec()).unwrap_or_default();

            Some(SampleEvent::new(
                time,
                ip,
                pid,
                tid,
                sample_period,
                callchain,
                cpu,
            ))
        }
        _ => None,
    }
}

fn record_system_wide(
    cpus: Vec<u32>,
    event: Hardware,
    sample_period: u64,
    output_path: &str,
) -> Result<()> {
    eprintln!("Recording on CPUs: {:?}", cpus);

    let mut ringbufs = Vec::with_capacity(cpus.len());
    for &cpu in &cpus {
        let ringbuf =
            crate::core::ringbuf::RingBuffer::from_event_for_cpu(event, cpu, sample_period, false)
                .with_context(|| format!("Failed to create ring buffer for CPU {}", cpu))?;
        ringbufs.push((cpu, ringbuf));
    }

    for (cpu, ringbuf) in &mut ringbufs {
        ringbuf
            .enable()
            .with_context(|| format!("Failed to enable ring buffer for CPU {}", cpu))?;
    }

    let mut writer = PerfDataWriter::from_path(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path))?;

    let start_time = Instant::now();
    let mut sample_count = 0u64;
    let mut total_lost = 0u64;

    eprintln!("Recording... Press Ctrl+C to stop.");

    while !SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
        for (_cpu, ringbuf) in &mut ringbufs {
            while let Some(record) = ringbuf.next_record() {
                if let Some(sample) = parse_sample_record(&record, sample_period) {
                    writer
                        .write_sample(&sample)
                        .context("Failed to write sample")?;
                    sample_count += 1;
                }
            }
        }

        std::thread::sleep(Duration::from_micros(100));
    }

    for (_cpu, ringbuf) in &mut ringbufs {
        ringbuf.disable().ok();
        total_lost += ringbuf.lost_count();
    }

    writer
        .finalize_with_header_update()
        .context("Failed to finalize output file")?;

    let elapsed = start_time.elapsed();
    eprintln!(
        "Recorded {} samples in {:.2}s ({} lost)",
        sample_count,
        elapsed.as_secs_f64(),
        total_lost
    );
    eprintln!("Saved to: {}", output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event() {
        assert!(matches!(
            parse_event("cpu-cycles"),
            Ok(Hardware::CPU_CYCLES)
        ));
        assert!(matches!(parse_event("cycles"), Ok(Hardware::CPU_CYCLES)));
        assert!(matches!(
            parse_event("instructions"),
            Ok(Hardware::INSTRUCTIONS)
        ));
        assert!(parse_event("cache-misses").is_ok());
        assert!(parse_event("unknown").is_err());
    }

    #[test]
    fn test_execute_no_command() {
        let result = execute(None, false, None, None, None, None, None, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No command specified"));
    }

    #[test]
    fn test_process_exists() {
        assert!(process_exists(1));
        assert!(!process_exists(999999999));
    }

    #[test]
    fn test_privilege_check() {
        let _ = check_privilege();
    }
}
