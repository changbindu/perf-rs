//! Performance statistics command - counts performance events for command execution.

use crate::core::perf_event::{
    create_counter, disable_counter, enable_counter, read_counter, Hardware, PerfConfig,
};
use crate::core::privilege::check_privilege;
use crate::error::PerfError;
use anyhow::{Context, Result};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{execvp, fork, ForkResult, Pid};

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

    if pid.is_some() {
        return Err(anyhow::anyhow!(
            "--pid mode is not yet implemented. Use stat -- <command> instead."
        ));
    }

    if command.is_empty() {
        return Err(anyhow::anyhow!(
            "No command specified. Usage: perf stat -- <command> [args...]"
        ));
    }

    if event.is_some() {
        eprintln!("Warning: Custom events not yet supported. Using default events (cpu-cycles, instructions).");
    }

    run_with_counters(command)
}

/// Run a command with default performance counters (cpu-cycles, instructions).
fn run_with_counters(command: &[String]) -> Result<()> {
    match unsafe { fork() }? {
        ForkResult::Parent { child } => {
            waitpid(child, Some(WaitPidFlag::WUNTRACED))
                .context("Failed to wait for child to stop")?;

            let config = PerfConfig::new().with_pid(child.as_raw() as u32);

            let mut cycles_counter = create_counter(Hardware::CPU_CYCLES, &config)
                .context("Failed to create cpu-cycles counter")?;

            let mut instructions_counter = create_counter(Hardware::INSTRUCTIONS, &config)
                .context("Failed to create instructions counter")?;

            enable_counter(&mut cycles_counter, "cpu-cycles")
                .context("Failed to enable cpu-cycles counter")?;
            enable_counter(&mut instructions_counter, "instructions")
                .context("Failed to enable instructions counter")?;

            kill(child, Signal::SIGCONT).context("Failed to continue child process")?;

            let status = waitpid(child, None).context("Failed to wait for child process")?;

            disable_counter(&mut cycles_counter, "cpu-cycles").ok();
            disable_counter(&mut instructions_counter, "instructions").ok();

            let cycles = read_counter(&mut cycles_counter, "cpu-cycles")
                .context("Failed to read cpu-cycles counter")?;
            let instructions = read_counter(&mut instructions_counter, "instructions")
                .context("Failed to read instructions counter")?;

            display_results(cycles, instructions, command, status);

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
fn display_results(cycles: u64, instructions: u64, command: &[String], status: WaitStatus) {
    println!("\n Performance counter stats for '{}':", command.join(" "));
    println!();

    println!("  {:>16}  cpu-cycles", format_number(cycles));
    println!("  {:>16}  instructions", format_number(instructions));

    if cycles > 0 {
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
    fn test_execute_pid_not_implemented() {
        let result = execute(Some(1234), None, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("--pid mode is not yet implemented"));
    }

    #[test]
    fn test_privilege_check() {
        let _ = check_privilege();
    }
}
