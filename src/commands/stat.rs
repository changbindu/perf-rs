//! Performance statistics command - counts performance events for command execution.

use crate::core::cpu::{get_online_cpus, parse_cpu_list, validate_cpu_ids};
use crate::core::perf_event::{
    create_counter, disable_counter, enable_counter, read_counter, PerfConfig,
};
use crate::core::privilege::check_privilege;
use crate::error::PerfError;
use crate::events::{format_parsed_event_name, parse_events, Hardware, ParsedEvent, PerfEvent};
use anyhow::{Context, Result};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{execvp, fork, ForkResult, Pid};

/// Execute the stat command.
pub fn execute(
    pid: Option<u32>,
    event: Option<&str>,
    all_cpus: bool,
    cpu: Option<&str>,
    per_cpu: bool,
    command: &[String],
) -> Result<()> {
    let is_system_wide = all_cpus || cpu.is_some();
    if pid.is_none() && !is_system_wide && command.is_empty() {
        return Err(anyhow::anyhow!(
            "No command specified. Usage: perf stat -- <command> [args...]"
        ));
    }

    let privilege_level = check_privilege().map_err(|e| PerfError::PermissionDenied {
        operation: e.to_string(),
    })?;

    if is_system_wide {
        if !privilege_level.can_profile_system_wide() {
            eprintln!("Error: Insufficient privileges for system-wide performance monitoring.");
            for suggestion in privilege_level.suggestions() {
                eprintln!("  {}", suggestion);
            }
            return Err(PerfError::PermissionDenied {
                operation: "system-wide performance monitoring".to_string(),
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
        vec![
            ParsedEvent::new(PerfEvent::Hardware(Hardware::CPU_CYCLES)),
            ParsedEvent::new(PerfEvent::Hardware(Hardware::INSTRUCTIONS)),
        ]
    };

    if events.is_empty() {
        return Err(anyhow::anyhow!(
            "No events specified. Usage: perf stat -e <events> -- <command>"
        ));
    }

    if is_system_wide {
        let cpus = if all_cpus {
            get_online_cpus().context("Failed to get online CPUs")?
        } else {
            // SAFETY: is_system_wide is true and all_cpus is false, so cpu must be Some
            let cpu_str = cpu.unwrap();
            let cpus = parse_cpu_list(cpu_str).context("Failed to parse CPU list")?;
            let online_cpus = get_online_cpus().context("Failed to get online CPUs")?;
            let max_cpu = online_cpus.iter().max().copied().unwrap_or(0);
            validate_cpu_ids(&cpus, max_cpu).context("Invalid CPU ID in list")?;
            cpus
        };

        if cpus.is_empty() {
            return Err(anyhow::anyhow!("No CPUs available for monitoring"));
        }

        if let Some(_target_pid) = pid {
            eprintln!(
                "Warning: --pid specified with system-wide mode. Monitoring entire system instead."
            );
        }

        if command.is_empty() {
            run_system_wide_standalone(&cpus, &events, per_cpu)
        } else {
            run_system_wide_with_counters(&cpus, &events, command, per_cpu)
        }
    } else if let Some(target_pid) = pid {
        run_with_pid(target_pid, &events)
    } else {
        run_with_counters(command, &events)
    }
}

/// Attach to a running process and monitor its performance counters.
fn run_with_pid(pid: u32, events: &[ParsedEvent]) -> Result<()> {
    let mut counters: Vec<(String, perf_event::Counter)> = Vec::new();

    for parsed in events {
        // For tracepoints, we need to monitor system-wide (cpu 0) and include kernel events
        let config = if parsed.event.is_tracepoint() {
            PerfConfig::new()
                .with_cpu(0)
                .with_include_kernel(true)
                .with_modifiers(parsed.modifiers)
        } else {
            PerfConfig::new()
                .with_pid(pid)
                .with_inherit(true)
                .with_modifiers(parsed.modifiers)
        };

        let counter = create_counter(parsed.event.clone(), &config)
            .with_context(|| format!("Failed to create {:?} counter", parsed.event))?;
        counters.push((format_parsed_event_name(parsed), counter));
    }

    for (name, counter) in &mut counters {
        enable_counter(counter, name)
            .with_context(|| format!("Failed to enable {} counter", name))?;
    }

    eprintln!("Monitoring process {} for 1 second...", pid);
    std::thread::sleep(std::time::Duration::from_secs(1));

    for (name, counter) in &mut counters {
        disable_counter(counter, name).ok();
    }

    let mut values: Vec<(String, u64)> = Vec::new();
    for (name, counter) in &mut counters {
        let value = read_counter(counter, name)
            .with_context(|| format!("Failed to read {} counter", name))?;
        values.push((name.clone(), value));
    }

    display_results(
        &values,
        &[format!("pid {}", pid)],
        WaitStatus::Exited(Pid::from_raw(pid as i32), 0),
    );

    Ok(())
}

/// Run a command with performance counters.
fn run_with_counters(command: &[String], events: &[ParsedEvent]) -> Result<()> {
    match unsafe { fork() }? {
        ForkResult::Parent { child } => {
            waitpid(child, Some(WaitPidFlag::WUNTRACED))
                .context("Failed to wait for child to stop")?;

            let mut counters: Vec<(String, perf_event::Counter)> = Vec::new();

            for parsed in events {
                // For tracepoints, we need to monitor system-wide (cpu 0) and include kernel events
                let config = if parsed.event.is_tracepoint() {
                    PerfConfig::new()
                        .with_cpu(0)
                        .with_include_kernel(true)
                        .with_modifiers(parsed.modifiers)
                } else {
                    PerfConfig::new()
                        .with_pid(child.as_raw() as u32)
                        .with_inherit(true)
                        .with_modifiers(parsed.modifiers)
                };

                let counter = create_counter(parsed.event.clone(), &config)
                    .with_context(|| format!("Failed to create {:?} counter", parsed.event))?;
                counters.push((format_parsed_event_name(parsed), counter));
            }

            for (name, counter) in &mut counters {
                enable_counter(counter, name)
                    .with_context(|| format!("Failed to enable {} counter", name))?;
            }

            kill(child, Signal::SIGCONT).context("Failed to continue child process")?;

            let status = waitpid(child, None).context("Failed to wait for child process")?;

            for (name, counter) in &mut counters {
                disable_counter(counter, name).ok();
            }

            let mut values: Vec<(String, u64)> = Vec::new();
            for (name, counter) in &mut counters {
                let value = read_counter(counter, name)
                    .with_context(|| format!("Failed to read {} counter", name))?;
                values.push((name.clone(), value));
            }

            display_results(&values, command, status);

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

struct CpuCounter {
    cpu: u32,
    name: String,
    counter: perf_event::Counter,
}

fn create_per_cpu_counters(cpus: &[u32], events: &[ParsedEvent]) -> Result<Vec<CpuCounter>> {
    let mut counters = Vec::new();

    for &cpu in cpus {
        for parsed in events {
            let config = PerfConfig::new()
                .with_cpu(cpu)
                .with_modifiers(parsed.modifiers);
            match create_counter(parsed.event.clone(), &config) {
                Ok(counter) => {
                    counters.push(CpuCounter {
                        cpu,
                        name: format_parsed_event_name(parsed),
                        counter,
                    });
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to create {:?} counter for CPU {}: {}",
                        parsed.event, cpu, e
                    );
                }
            }
        }
    }

    if counters.is_empty() {
        return Err(anyhow::anyhow!(
            "Failed to create any counters. All CPUs may be offline or inaccessible."
        ));
    }

    Ok(counters)
}

fn aggregate_counter_values(counters: &mut [CpuCounter]) -> Vec<(String, u64)> {
    let mut aggregated: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    for cpu_counter in counters.iter_mut() {
        if let Ok(value) = read_counter(&mut cpu_counter.counter, &cpu_counter.name) {
            *aggregated.entry(cpu_counter.name.clone()).or_insert(0) += value;
        }
    }

    let mut result: Vec<(String, u64)> = aggregated.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

/// Read per-CPU counter values without aggregation.
fn read_per_cpu_values(counters: &mut [CpuCounter]) -> Vec<(u32, String, u64)> {
    let mut values = Vec::new();

    for cpu_counter in counters.iter_mut() {
        match read_counter(&mut cpu_counter.counter, &cpu_counter.name) {
            Ok(value) => values.push((cpu_counter.cpu, cpu_counter.name.clone(), value)),
            Err(e) => eprintln!(
                "Warning: Failed to read {} counter for CPU {}: {}",
                cpu_counter.name, cpu_counter.cpu, e
            ),
        }
    }

    // Sort by CPU ID (ascending), then by event name (alphabetical)
    values.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    values
}

/// Display per-CPU results in table format.
fn display_per_cpu_results(values: &[(u32, String, u64)], command: &[String], status: WaitStatus) {
    println!("\n Performance counter stats for '{}':", command.join(" "));
    println!();

    if values.is_empty() {
        println!("  No data available");
        println!();
        return;
    }

    // Calculate totals per event for percentage calculation
    let mut event_totals: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for (_, name, value) in values {
        *event_totals.entry(name.clone()).or_insert(0) += value;
    }

    // Display table header
    println!(
        "  {:>4}  {:<20} {:>16}  {:>10}",
        "CPU", "Event", "Count", "Overhead"
    );
    println!("  {}", "-".repeat(54));

    // Group values by CPU for display
    let mut current_cpu: Option<u32> = None;
    for (cpu, name, value) in values {
        // Add blank line between different CPUs
        if current_cpu.is_some() && current_cpu != Some(*cpu) {
            println!();
        }
        current_cpu = Some(*cpu);

        // Calculate percentage based on total for this event
        let percentage = if let Some(&total) = event_totals.get(name) {
            if total > 0 {
                (*value as f64 / total as f64) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        println!(
            "  {:>4}  {:<20} {:>16}  {:>9.2}%",
            cpu,
            name,
            format_number(*value),
            percentage
        );
    }

    println!();

    // Calculate overall IPC if we have cycles and instructions
    let cycles: u64 = values
        .iter()
        .filter(|(_, name, _)| name == "cpu-cycles")
        .map(|(_, _, v)| *v)
        .sum();
    let instructions: u64 = values
        .iter()
        .filter(|(_, name, _)| name == "instructions")
        .map(|(_, _, v)| *v)
        .sum();

    if cycles > 0 && instructions > 0 {
        let ipc = instructions as f64 / cycles as f64;
        println!("  {:>27}  {:.2}", "", ipc);
    }

    println!();

    match status {
        WaitStatus::Exited(_, code) => {
            if code != 0 {
                eprintln!("  Process exited with status: {}", code);
            }
        }
        WaitStatus::Signaled(_, signal, _) => {
            eprintln!("  Process terminated by signal: {:?}", signal);
        }
        _ => {}
    }
}

fn run_system_wide_standalone(cpus: &[u32], events: &[ParsedEvent], per_cpu: bool) -> Result<()> {
    let mut counters = create_per_cpu_counters(cpus, events)?;

    eprintln!(
        "Monitoring system-wide on {} CPU(s) for 1 second...",
        cpus.len()
    );

    for cpu_counter in &mut counters {
        enable_counter(&mut cpu_counter.counter, &cpu_counter.name).ok();
    }

    std::thread::sleep(std::time::Duration::from_secs(1));

    for cpu_counter in &mut counters {
        disable_counter(&mut cpu_counter.counter, &cpu_counter.name).ok();
    }

    if per_cpu {
        let values = read_per_cpu_values(&mut counters);
        display_per_cpu_results(
            &values,
            &["system-wide".to_string()],
            WaitStatus::Exited(Pid::from_raw(0), 0),
        );
    } else {
        let values = aggregate_counter_values(&mut counters);
        display_results(
            &values,
            &["system-wide".to_string()],
            WaitStatus::Exited(Pid::from_raw(0), 0),
        );
    }

    Ok(())
}

fn run_system_wide_with_counters(
    cpus: &[u32],
    events: &[ParsedEvent],
    command: &[String],
    per_cpu: bool,
) -> Result<()> {
    match unsafe { fork() }? {
        ForkResult::Parent { child } => {
            waitpid(child, Some(WaitPidFlag::WUNTRACED))
                .context("Failed to wait for child to stop")?;

            let mut counters = create_per_cpu_counters(cpus, events)?;

            for cpu_counter in &mut counters {
                enable_counter(&mut cpu_counter.counter, &cpu_counter.name).ok();
            }

            kill(child, Signal::SIGCONT).context("Failed to continue child process")?;

            let status = waitpid(child, None).context("Failed to wait for child process")?;

            for cpu_counter in &mut counters {
                disable_counter(&mut cpu_counter.counter, &cpu_counter.name).ok();
            }

            if per_cpu {
                let values = read_per_cpu_values(&mut counters);
                display_per_cpu_results(&values, command, status);
            } else {
                let values = aggregate_counter_values(&mut counters);
                display_results(&values, command, status);
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

/// Display the performance counter results.
fn display_results(values: &[(String, u64)], command: &[String], status: WaitStatus) {
    println!("\n Performance counter stats for '{}':", command.join(" "));
    println!();

    let mut sorted_values = values.to_vec();
    sorted_values.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, value) in &sorted_values {
        println!("  {:>16}  {}", format_number(*value), name);
    }

    let cycles = sorted_values
        .iter()
        .find(|(name, _)| name == "cpu-cycles")
        .map(|(_, v)| *v)
        .unwrap_or(0);
    let instructions = sorted_values
        .iter()
        .find(|(name, _)| name == "instructions")
        .map(|(_, v)| *v)
        .unwrap_or(0);

    if cycles > 0 && instructions > 0 {
        let ipc = instructions as f64 / cycles as f64;
        println!("  {:>16}  insn per cycle", format!("{:.2}", ipc));
    }

    println!();

    match status {
        WaitStatus::Exited(_, code) => {
            if code != 0 {
                eprintln!("  Process exited with status: {}", code);
            }
        }
        WaitStatus::Signaled(_, signal, _) => {
            eprintln!("  Process terminated by signal: {:?}", signal);
        }
        _ => {}
    }
}

/// Format a number with thousands separators.
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{format_event_name, parse_event};
    use crate::tracepoint::TracepointId;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(12345), "12,345");
        assert_eq!(format_number(123456), "123,456");
        assert_eq!(format_number(1234567), "1,234,567");
        assert_eq!(format_number(1000000000), "1,000,000,000");
    }

    #[test]
    fn test_execute_no_command() {
        let result = execute(None, None, false, None, false, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No command specified"));
    }

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
        let evt = parse_event("cache-misses").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::CACHE_MISSES)
        ));
        assert!(parse_event("unknown").is_err());
    }

    #[test]
    fn test_parse_events() {
        let events = parse_events("cpu-cycles,instructions").unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0].event,
            PerfEvent::Hardware(Hardware::CPU_CYCLES)
        ));
        assert!(matches!(
            events[1].event,
            PerfEvent::Hardware(Hardware::INSTRUCTIONS)
        ));
    }

    #[test]
    fn test_format_event_name() {
        assert_eq!(
            format_event_name(&PerfEvent::Hardware(Hardware::CPU_CYCLES)),
            "cpu-cycles"
        );
        assert_eq!(
            format_event_name(&PerfEvent::Hardware(Hardware::INSTRUCTIONS)),
            "instructions"
        );
        assert_eq!(
            format_event_name(&PerfEvent::Hardware(Hardware::CACHE_MISSES)),
            "cache-misses"
        );
    }

    #[test]
    fn test_privilege_check() {
        let _ = check_privilege();
    }

    #[test]
    fn test_parse_tracepoint_event_recognizes_format() {
        let result = parse_event("sched:sched_switch");
        match result {
            Ok(parsed) => match parsed.event {
                PerfEvent::Tracepoint(tp) => {
                    assert_eq!(tp.subsystem, "sched");
                    assert_eq!(tp.name, "sched_switch");
                }
                _ => panic!("Expected Tracepoint variant"),
            },
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    err_msg.contains("not found")
                        || err_msg.contains("tracefs")
                        || err_msg.contains("Permission"),
                    "Unexpected error: {}",
                    err_msg
                );
            }
        }
    }

    #[test]
    fn test_parse_tracepoint_invalid_format_returns_error() {
        let result = parse_event("invalid_no_colon");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_events_with_tracepoint_mixed() {
        let result = parse_events("cpu-cycles,sched:sched_switch");
        match result {
            Ok(events) => {
                assert!(events.len() >= 1);
                if events.len() == 2 {
                    assert!(matches!(events[0].event, PerfEvent::Hardware(_)));
                    assert!(matches!(events[1].event, PerfEvent::Tracepoint(_)));
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    err_msg.contains("not found") || err_msg.contains("tracefs"),
                    "Unexpected error: {}",
                    err_msg
                );
            }
        }
    }

    #[test]
    fn test_format_tracepoint_event_name() {
        let tp = TracepointId::new("sched", "sched_switch", 123);
        let event = PerfEvent::Tracepoint(tp);
        assert_eq!(format_event_name(&event), "sched:sched_switch");
    }

    #[test]
    fn test_tracepoint_is_tracepoint_method() {
        let tp = TracepointId::new("irq", "irq_handler_entry", 456);
        let event = PerfEvent::Tracepoint(tp);
        assert!(event.is_tracepoint());
        assert!(!event.is_hardware());
    }
}
