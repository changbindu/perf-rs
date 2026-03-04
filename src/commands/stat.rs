//! Performance statistics command - counts performance events for command execution.

use crate::core::perf_event::{
    add_to_group, create_group_with_config, disable_group, enable_group, read_group, Hardware,
    PerfConfig,
};
use crate::core::privilege::check_privilege;
use crate::error::PerfError;
use anyhow::{Context, Result};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{execvp, fork, ForkResult, Pid};
use perf_event::GroupData;
use std::collections::HashMap;

/// Parse event name string to Hardware enum.
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

/// Parse comma-separated event string into vector of Hardware events.
fn parse_events(events_str: &str) -> Result<Vec<Hardware>> {
    let events: Result<Vec<Hardware>> = events_str
        .split(',')
        .map(|s| parse_event(s.trim()))
        .collect();
    events
}

/// Execute the stat command.
pub fn execute(pid: Option<u32>, event: Option<&str>, command: &[String]) -> Result<()> {
    let privilege_level = check_privilege().map_err(|e| PerfError::PermissionDenied {
        operation: e.to_string(),
    })?;

    if !privilege_level.can_profile() {
        eprintln!("Error: Insufficient privileges for performance monitoring.");
        for suggestion in privilege_level.suggestions() {
            eprintln!("  {}", suggestion);
        }
        std::process::exit(1);
    }

    let events = if let Some(events_str) = event {
        parse_events(events_str)?
    } else {
        vec![Hardware::CPU_CYCLES, Hardware::INSTRUCTIONS]
    };

    if events.is_empty() {
        return Err(anyhow::anyhow!(
            "No events specified. Usage: perf stat -e <events> -- <command>"
        ));
    }

    if let Some(target_pid) = pid {
        run_with_pid(target_pid, &events)
    } else if command.is_empty() {
        Err(anyhow::anyhow!(
            "No command specified. Usage: perf stat -- <command> [args...]"
        ))
    } else {
        run_with_counters(command, &events)
    }
}

/// Attach to a running process and monitor its performance counters.
fn run_with_pid(pid: u32, events: &[Hardware]) -> Result<()> {
    let config = PerfConfig::new().with_pid(pid).with_inherit(true);

    let mut group = create_group_with_config(&config).context("Failed to create counter group")?;

    let mut event_names: HashMap<u64, String> = HashMap::new();

    for event in events {
        let counter = add_to_group(&mut group, *event, &config)
            .with_context(|| format!("Failed to add {:?} to group", event))?;
        event_names.insert(counter.id(), format_event_name(event));
    }

    enable_group(&mut group).context("Failed to enable counter group")?;

    eprintln!("Monitoring process {} for 1 second...", pid);
    std::thread::sleep(std::time::Duration::from_secs(1));

    disable_group(&mut group).ok();

    let group_data = read_group(&mut group).context("Failed to read counter group")?;

    display_results(
        &group_data,
        &event_names,
        &[format!("pid {}", pid)],
        WaitStatus::Exited(Pid::from_raw(pid as i32), 0),
    );

    Ok(())
}

/// Format a Hardware event enum to a human-readable name.
fn format_event_name(event: &Hardware) -> String {
    match *event {
        Hardware::CPU_CYCLES => "cpu-cycles".to_string(),
        Hardware::INSTRUCTIONS => "instructions".to_string(),
        Hardware::CACHE_REFERENCES => "cache-references".to_string(),
        Hardware::CACHE_MISSES => "cache-misses".to_string(),
        Hardware::BRANCH_INSTRUCTIONS => "branch-instructions".to_string(),
        Hardware::BRANCH_MISSES => "branch-misses".to_string(),
        Hardware::STALLED_CYCLES_FRONTEND => "stalled-cycles-frontend".to_string(),
        Hardware::STALLED_CYCLES_BACKEND => "stalled-cycles-backend".to_string(),
        Hardware::REF_CPU_CYCLES => "ref-cpu-cycles".to_string(),
        _ => format!("hardware-event-{:?}", event),
    }
}

/// Run a command with performance counters.
fn run_with_counters(command: &[String], events: &[Hardware]) -> Result<()> {
    match unsafe { fork() }? {
        ForkResult::Parent { child } => {
            waitpid(child, Some(WaitPidFlag::WUNTRACED))
                .context("Failed to wait for child to stop")?;

            let config = PerfConfig::new()
                .with_pid(child.as_raw() as u32)
                .with_inherit(true);

            let mut group =
                create_group_with_config(&config).context("Failed to create counter group")?;

            let mut event_names: HashMap<u64, String> = HashMap::new();

            for event in events {
                let counter = add_to_group(&mut group, *event, &config)
                    .with_context(|| format!("Failed to add {:?} to group", event))?;
                event_names.insert(counter.id(), format_event_name(event));
            }

            enable_group(&mut group).context("Failed to enable counter group")?;

            kill(child, Signal::SIGCONT).context("Failed to continue child process")?;

            let status = waitpid(child, None).context("Failed to wait for child process")?;

            disable_group(&mut group).ok();

            let group_data = read_group(&mut group).context("Failed to read counter group")?;

            display_results(&group_data, &event_names, command, status);

            Ok(())
        }
        ForkResult::Child => {
            kill(Pid::this(), Signal::SIGSTOP).ok();

            let program = &command[0];
            let args: Vec<std::ffi::CString> = command
                .iter()
                .map(|s| std::ffi::CString::new(s.as_bytes()).unwrap())
                .collect();

            execvp(
                std::ffi::CString::new(program.as_bytes())
                    .unwrap()
                    .as_c_str(),
                &args.iter().map(|s| s.as_c_str()).collect::<Vec<_>>(),
            )
            .context("Failed to execute command")?;

            unreachable!()
        }
    }
}

/// Display the performance counter results.
fn display_results(
    group_data: &GroupData,
    event_names: &HashMap<u64, String>,
    command: &[String],
    status: WaitStatus,
) {
    println!("\n Performance counter stats for '{}':", command.join(" "));
    println!();

    let mut values: Vec<(String, u64)> = Vec::new();

    for entry in group_data.iter() {
        if let Some(name) = event_names.get(&entry.id()) {
            values.push((name.clone(), entry.value()));
        }
    }

    values.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, value) in &values {
        println!("  {:>16}  {}", format_number(*value), name);
    }

    let cycles = values
        .iter()
        .find(|(name, _)| name == "cpu-cycles")
        .map(|(_, v)| *v)
        .unwrap_or(0);
    let instructions = values
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
        let result = execute(None, None, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No command specified"));
    }

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
        assert!(matches!(
            parse_event("cache-misses"),
            Ok(Hardware::CACHE_MISSES)
        ));
        assert!(parse_event("unknown").is_err());
    }

    #[test]
    fn test_parse_events() {
        let events = parse_events("cpu-cycles,instructions").unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Hardware::CPU_CYCLES));
        assert!(matches!(events[1], Hardware::INSTRUCTIONS));
    }

    #[test]
    fn test_format_event_name() {
        assert_eq!(format_event_name(&Hardware::CPU_CYCLES), "cpu-cycles");
        assert_eq!(format_event_name(&Hardware::INSTRUCTIONS), "instructions");
        assert_eq!(format_event_name(&Hardware::CACHE_MISSES), "cache-misses");
    }

    #[test]
    fn test_privilege_check() {
        let _ = check_privilege();
    }
}
