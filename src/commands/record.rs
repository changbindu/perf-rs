//! Performance recording command - collects samples for profiling.

use crate::core::cpu::{get_online_cpus, parse_cpu_list, validate_cpu_ids};
use crate::core::perf_data::{CommEvent, MmapEvent, PerfDataWriter, PerfEventAttr, SampleEvent};
use crate::core::privilege::check_privilege;
use crate::core::ringbuf::RingBuffer;
use crate::error::PerfError;
use crate::events::{parse_events, Hardware, ParsedEvent, PerfEvent};
use anyhow::{Context, Result};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{execvp, fork, ForkResult, Pid};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);
static EVENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Associates a ring buffer with its event metadata for multi-event recording.
struct EventRingBuffer {
    event_id: u64,
    ringbuf: RingBuffer,
    sample_period: u64,
    sample_type: u64,
}

extern "C" fn signal_handler(_sig: libc::c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

/// Get the effective sample period for an event.
/// Tracepoints use period=1 to capture every occurrence.
fn effective_sample_period(event: &PerfEvent, sample_period: u64) -> u64 {
    if matches!(event, PerfEvent::Tracepoint(_)) {
        1
    } else {
        sample_period
    }
}

/// Convert a PerfEvent to PerfEventAttr for Linux perf format
fn event_to_attr(event: &PerfEvent, sample_period: u64, callchain: bool) -> PerfEventAttr {
    let (attr_type, config) = match event {
        PerfEvent::Hardware(h) => (0u32, u64::from(*h)),
        PerfEvent::Software(s) => (1u32, u64::from(*s)),
        PerfEvent::Tracepoint(t) => (2u32, t.id),
        PerfEvent::Cache(c) => {
            let config =
                c.which.0 as u64 | ((c.operation.0 as u64) << 8) | ((c.result.0 as u64) << 16);
            (3u32, config)
        }
        PerfEvent::Raw(r) => (4u32, r.config),
    };

    let effective_period = effective_sample_period(event, sample_period);

    let mut sample_type = crate::core::perf_data::PERF_SAMPLE_IP
        | crate::core::perf_data::PERF_SAMPLE_TID
        | crate::core::perf_data::PERF_SAMPLE_TIME
        | crate::core::perf_data::PERF_SAMPLE_PERIOD
        | crate::core::perf_data::PERF_SAMPLE_IDENTIFIER;

    if callchain {
        sample_type |= crate::core::perf_data::PERF_SAMPLE_CALLCHAIN;
    }

    PerfEventAttr::new(attr_type, config, sample_type)
        .with_sample_period(effective_period)
        .with_comm(true)
        .with_mmap(true)
        .with_sample_id_all(true)
}

/// Get process command name from /proc/PID/comm
fn get_process_comm(pid: u32) -> Result<String> {
    let comm_path = format!("/proc/{}/comm", pid);
    std::fs::read_to_string(&comm_path)
        .with_context(|| format!("Failed to read {}", comm_path))
        .map(|s| s.trim().to_string())
}

/// Generate a fake event ID for the attribute
/// In real perf, this is obtained from perf_event_id system call
fn generate_event_id() -> u64 {
    let counter = EVENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    1024 + (counter % 65536)
}

/// Write COMM and MMAP events for a process
fn write_process_events(
    writer: &mut PerfDataWriter<impl std::io::Write + std::io::Seek>,
    pid: u32,
    tid: u32,
) -> Result<()> {
    if let Ok(comm) = get_process_comm(pid) {
        let comm_event = CommEvent::new(pid, tid, comm);
        writer
            .write_comm(&comm_event)
            .context("Failed to write COMM event")?;
    }

    let maps_path = format!("/proc/{}/maps", pid);
    if let Ok(maps_content) = std::fs::read_to_string(maps_path) {
        for line in maps_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                if let Some(addr_range) = parts.first() {
                    if let Some((start_str, end_str)) = addr_range.split_once('-') {
                        if let (Ok(start), Ok(end)) = (
                            u64::from_str_radix(start_str, 16),
                            u64::from_str_radix(end_str, 16),
                        ) {
                            let addr = start;
                            let len = end - start;
                            let offset = 0;
                            let filename = parts.get(5).unwrap_or(&"").to_string();

                            if !filename.is_empty() && !filename.starts_with('[') {
                                let mmap_event =
                                    MmapEvent::new(pid, tid, addr, len, offset, filename);
                                writer
                                    .write_mmap(&mmap_event)
                                    .context("Failed to write MMAP event")?;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
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
    call_graph: Option<u16>,
    command: &[String],
) -> Result<()> {
    let is_system_wide = all_cpus || cpu.is_some();
    if pid.is_none() && !is_system_wide && command.is_empty() {
        return Err(anyhow::anyhow!(
            "No command specified. Usage: perf record -- <command> [args...]"
        ));
    }

    let privilege_level = check_privilege().map_err(|e| PerfError::PermissionDenied {
        operation: e.to_string(),
    })?;

    if is_system_wide {
        if !privilege_level.can_profile_system_wide() {
            eprintln!("Error: Insufficient privileges for system-wide profiling.");
            for suggestion in privilege_level.suggestions() {
                eprintln!("  {}", suggestion);
            }
            return Err(PerfError::PermissionDenied {
                operation: "system-wide profiling".to_string(),
            }
            .into());
        }
    } else if !privilege_level.can_profile() {
        eprintln!("Error: Insufficient privileges for performance monitoring.");
        for suggestion in privilege_level.suggestions() {
            eprintln!("  {}", suggestion);
        }
        return Err(PerfError::PermissionDenied {
            operation: "performance monitoring".to_string(),
        }
        .into());
    }

    let events = if let Some(events_str) = event {
        parse_events(events_str)?
    } else {
        vec![ParsedEvent::new(PerfEvent::Hardware(Hardware::CPU_CYCLES))]
    };

    if events.is_empty() {
        return Err(anyhow::anyhow!(
            "No events specified. Usage: perf record -e <events> -- <command>"
        ));
    }

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

        record_system_wide(cpus, events, sample_period, output_path, call_graph)
    } else if let Some(target_pid) = pid {
        record_with_pid(target_pid, events, sample_period, output_path, call_graph)
    } else {
        record_with_command(command, events, sample_period, output_path, call_graph)
    }
}

fn record_with_pid(
    pid: u32,
    events: Vec<ParsedEvent>,
    sample_period: u64,
    output_path: &str,
    call_graph: Option<u16>,
) -> Result<()> {
    let callchain = call_graph.is_some();
    let max_stack = call_graph.unwrap_or(0);

    eprintln!("Recording process {} ...", pid);

    let mut writer = PerfDataWriter::from_path(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path))?;

    let mut attrs = Vec::with_capacity(events.len());
    let mut all_event_ids = Vec::with_capacity(events.len());
    let mut event_ringbufs: Vec<EventRingBuffer> = Vec::new();

    for parsed in events.iter() {
        let attr = event_to_attr(&parsed.event, sample_period, callchain);
        let sample_type = attr.sample_type;
        let effective_period = effective_sample_period(&parsed.event, sample_period);
        let event_id = generate_event_id();

        let cpu = if parsed.event.is_tracepoint() {
            Some(0)
        } else {
            None
        };

        let ringbuf = RingBuffer::from_event_for_pid(
            parsed.event.clone(),
            pid as i32,
            effective_period,
            false,
            callchain,
            max_stack,
            cpu,
        )
        .with_context(|| format!("Failed to create ring buffer for event {:?}", parsed.event))?;

        attrs.push(attr);
        all_event_ids.push(vec![event_id]);
        event_ringbufs.push(EventRingBuffer {
            event_id,
            ringbuf,
            sample_period: effective_period,
            sample_type,
        });
    }

    writer
        .initialize(&attrs, &all_event_ids)
        .context("Failed to initialize perf.data file")?;

    write_process_events(&mut writer, pid, pid).context("Failed to write process events")?;

    for erb in &mut event_ringbufs {
        erb.ringbuf.enable().context("Failed to enable sampling")?;
    }

    let start_time = Instant::now();
    let mut sample_count = 0u64;
    let mut total_lost = 0u64;

    while !SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
        if !process_exists(pid) {
            break;
        }

        for erb in &mut event_ringbufs {
            while let Some(record) = erb.ringbuf.next_record() {
                if let Some(sample) =
                    parse_sample_record(&record, erb.sample_period, erb.sample_type, erb.event_id)
                {
                    writer
                        .write_sample(&sample)
                        .context("Failed to write sample")?;
                    sample_count += 1;
                }
            }
        }

        std::thread::sleep(Duration::from_micros(100));
    }

    for erb in &mut event_ringbufs {
        erb.ringbuf.disable().ok();
        total_lost += erb.ringbuf.lost_count();
    }

    writer
        .write_finished_round()
        .context("Failed to write FINISHED_ROUND event")?;
    writer
        .finalize()
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

fn record_with_command(
    command: &[String],
    events: Vec<ParsedEvent>,
    sample_period: u64,
    output_path: &str,
    call_graph: Option<u16>,
) -> Result<()> {
    let callchain = call_graph.is_some();
    let max_stack = call_graph.unwrap_or(0);

    match unsafe { fork() }? {
        ForkResult::Parent { child } => {
            waitpid(child, Some(WaitPidFlag::WUNTRACED))
                .context("Failed to wait for child to stop")?;

            let mut writer = PerfDataWriter::from_path(output_path)
                .with_context(|| format!("Failed to create output file: {}", output_path))?;

            let mut attrs = Vec::with_capacity(events.len());
            let mut all_event_ids = Vec::with_capacity(events.len());
            let mut event_ringbufs: Vec<EventRingBuffer> = Vec::new();

            for parsed in events.iter() {
                let attr = event_to_attr(&parsed.event, sample_period, callchain);
                let sample_type = attr.sample_type;
                let effective_period = effective_sample_period(&parsed.event, sample_period);
                let event_id = generate_event_id();

                let cpu = if parsed.event.is_tracepoint() {
                    Some(0)
                } else {
                    None
                };

                let ringbuf = RingBuffer::from_event_for_pid(
                    parsed.event.clone(),
                    child.as_raw(),
                    effective_period,
                    false,
                    callchain,
                    max_stack,
                    cpu,
                )
                .with_context(|| {
                    format!("Failed to create ring buffer for event {:?}", parsed.event)
                })?;

                attrs.push(attr);
                all_event_ids.push(vec![event_id]);
                event_ringbufs.push(EventRingBuffer {
                    event_id,
                    ringbuf,
                    sample_period: effective_period,
                    sample_type,
                });
            }

            writer
                .initialize(&attrs, &all_event_ids)
                .context("Failed to initialize perf.data file")?;

            write_process_events(&mut writer, child.as_raw() as u32, child.as_raw() as u32)
                .context("Failed to write process events")?;

            for erb in &mut event_ringbufs {
                erb.ringbuf.enable().context("Failed to enable sampling")?;
            }

            kill(child, Signal::SIGCONT).context("Failed to continue child process")?;

            let start_time = Instant::now();
            let mut sample_count = 0u64;
            let mut total_lost = 0u64;

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

                for erb in &mut event_ringbufs {
                    while let Some(record) = erb.ringbuf.next_record() {
                        if let Some(sample) = parse_sample_record(
                            &record,
                            erb.sample_period,
                            erb.sample_type,
                            erb.event_id,
                        ) {
                            writer
                                .write_sample(&sample)
                                .context("Failed to write sample")?;
                            sample_count += 1;
                        }
                    }
                }

                std::thread::sleep(Duration::from_micros(100));
            }

            let status = waitpid(child, None).ok();

            for erb in &mut event_ringbufs {
                erb.ringbuf.disable().ok();
                total_lost += erb.ringbuf.lost_count();
            }

            writer
                .write_finished_round()
                .context("Failed to write FINISHED_ROUND event")?;
            writer
                .finalize()
                .context("Failed to finalize output file")?;

            let elapsed = start_time.elapsed();
            eprintln!(
                "Recorded {} samples in {:.2}s ({} lost)",
                sample_count,
                elapsed.as_secs_f64(),
                total_lost
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

fn parse_sample_record(
    record: &perf_event::Record<'_>,
    sample_period: u64,
    sample_type: u64,
    event_id: u64,
) -> Option<SampleEvent> {
    use perf_event::data::Record as DataRecord;

    let parsed = record.parse_record().ok()?;

    match parsed {
        DataRecord::Sample(sample) => {
            let ip = sample.ip()?;
            let pid = sample.pid()?;
            let tid = sample.tid()?;
            let time = sample.time().unwrap_or(0);
            let cpu = sample.cpu();

            let callchain = sample.callchain().map(|cc| cc.to_vec());

            Some(SampleEvent::new(
                sample_type,
                time,
                ip,
                pid,
                tid,
                sample_period,
                callchain,
                cpu,
                event_id,
            ))
        }
        _ => None,
    }
}

fn record_system_wide(
    cpus: Vec<u32>,
    events: Vec<ParsedEvent>,
    sample_period: u64,
    output_path: &str,
    call_graph: Option<u16>,
) -> Result<()> {
    let callchain = call_graph.is_some();
    let max_stack = call_graph.unwrap_or(0);

    eprintln!("Recording on CPUs: {:?}", cpus);

    let mut writer = PerfDataWriter::from_path(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path))?;

    let mut attrs = Vec::with_capacity(events.len());
    let mut all_event_ids: Vec<Vec<u64>> = Vec::with_capacity(events.len());
    let mut event_ringbufs: Vec<(u64, RingBuffer, u64, u64)> = Vec::new();

    for parsed in events.iter() {
        let attr = event_to_attr(&parsed.event, sample_period, callchain);
        let sample_type = attr.sample_type;
        let effective_period = effective_sample_period(&parsed.event, sample_period);

        let mut event_ids_for_attr = Vec::with_capacity(cpus.len());

        for &cpu in &cpus {
            let event_id = generate_event_id();
            let ringbuf = RingBuffer::from_event_for_cpu(
                parsed.event.clone(),
                cpu,
                effective_period,
                false,
                callchain,
                max_stack,
            )
            .with_context(|| {
                format!(
                    "Failed to create ring buffer for CPU {} and event {:?}",
                    cpu, parsed.event
                )
            })?;

            event_ids_for_attr.push(event_id);
            event_ringbufs.push((event_id, ringbuf, effective_period, sample_type));
        }

        attrs.push(attr);
        all_event_ids.push(event_ids_for_attr);
    }

    writer
        .initialize(&attrs, &all_event_ids)
        .context("Failed to initialize perf.data file")?;

    for (_, ringbuf, _, _) in &mut event_ringbufs {
        ringbuf.enable().context("Failed to enable sampling")?;
    }

    let start_time = Instant::now();
    let mut sample_count = 0u64;
    let mut total_lost = 0u64;
    let mut seen_processes: std::collections::HashSet<(u32, u32)> =
        std::collections::HashSet::new();

    eprintln!("Recording... Press Ctrl+C to stop.");

    while !SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
        for (event_id, ringbuf, sample_period, sample_type) in &mut event_ringbufs {
            while let Some(record) = ringbuf.next_record() {
                if let Some(sample) =
                    parse_sample_record(&record, *sample_period, *sample_type, *event_id)
                {
                    let process_key = (sample.pid, sample.tid);
                    if seen_processes.insert(process_key) {
                        if let Err(e) = write_process_events(&mut writer, sample.pid, sample.tid) {
                            eprintln!(
                                "Warning: Failed to write process events for PID {}: {}",
                                sample.pid, e
                            );
                        }
                    }

                    writer
                        .write_sample(&sample)
                        .context("Failed to write sample")?;
                    sample_count += 1;
                }
            }
        }

        std::thread::sleep(Duration::from_micros(100));
    }

    for (_, ringbuf, _, _) in &mut event_ringbufs {
        ringbuf.disable().ok();
        total_lost += ringbuf.lost_count();
    }

    writer
        .write_finished_round()
        .context("Failed to write FINISHED_ROUND event")?;
    writer
        .finalize()
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
    use crate::events::parse_event;

    #[test]
    fn test_parse_event() {
        let evt = parse_event("cpu-cycles").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::CPU_CYCLES)
        ));
        let evt = parse_event("cycles").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::CPU_CYCLES)
        ));
        let evt = parse_event("instructions").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::INSTRUCTIONS)
        ));
        assert!(parse_event("cache-misses").is_ok());
        assert!(parse_event("cpu-clock").is_ok());
        assert!(parse_event("L1-dcache-loads").is_ok());
        assert!(parse_event("unknown").is_err());
    }

    #[test]
    fn test_execute_no_command() {
        let result = execute(None, false, None, None, None, None, None, None, &[]);
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
