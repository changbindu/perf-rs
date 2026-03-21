//! List available performance events
//!
//! This module implements the `perf list` command which displays available
//! hardware, software, and tracepoint performance events.

use std::collections::BTreeMap;
use std::io::Write;

use anyhow::{Context, Result};

use crate::arch;
use crate::pager::Pager;
use crate::tracepoint;

/// Event information for display
struct EventInfo {
    name: String,
    aliases: Vec<String>,
    category: String,
    description: String,
    detailed_description: String,
}

impl From<arch::PmuEvent> for EventInfo {
    fn from(event: arch::PmuEvent) -> Self {
        EventInfo {
            name: event.name,
            aliases: event.aliases,
            category: event.category,
            description: event.description.clone(),
            detailed_description: event.description,
        }
    }
}

/// Get all hardware events (architecture-specific)
fn get_hardware_events() -> Vec<EventInfo> {
    let arch_events = arch::get_arch_events();
    arch_events.into_iter().map(EventInfo::from).collect()
}

/// Get all software events
fn get_software_events() -> Vec<EventInfo> {
    vec![
        EventInfo {
            name: "cpu-clock".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "High-resolution per-CPU timer".to_string(),
            detailed_description: "High-resolution per-CPU timer. This measures CPU time using the CPU's high-resolution timer.".to_string(),
        },
        EventInfo {
            name: "task-clock".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "Per-task clock count".to_string(),
            detailed_description: "Per-task clock count. This measures the time a task is running on a CPU.".to_string(),
        },
        EventInfo {
            name: "page-faults".to_string(),
            aliases: vec!["faults".to_string()],
            category: "Software event".to_string(),
            description: "Page faults".to_string(),
            detailed_description: "Page faults. This counts both minor and major page faults.".to_string(),
        },
        EventInfo {
            name: "context-switches".to_string(),
            aliases: vec!["cs".to_string()],
            category: "Software event".to_string(),
            description: "Context switches".to_string(),
            detailed_description: "Context switches. This counts the number of times the CPU switched from one task to another.".to_string(),
        },
        EventInfo {
            name: "cpu-migrations".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "Process migration to another CPU".to_string(),
            detailed_description: "Process migration to another CPU. This counts when a process moves from one CPU to another.".to_string(),
        },
        EventInfo {
            name: "minor-faults".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "Minor page faults (resolved without I/O)".to_string(),
            detailed_description: "Minor page faults. These are page faults that can be resolved without disk I/O, typically by mapping an existing page in memory.".to_string(),
        },
        EventInfo {
            name: "major-faults".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "Major page faults (I/O required)".to_string(),
            detailed_description: "Major page faults. These are page faults that require disk I/O to resolve, typically loading a page from swap or a file.".to_string(),
        },
        EventInfo {
            name: "alignment-faults".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "Alignment faults (kernel intervention required)".to_string(),
            detailed_description: "Alignment faults that required kernel intervention. Note: This is only generated on some CPUs, never on x86_64 or ARM.".to_string(),
        },
        EventInfo {
            name: "emulation-faults".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "Instruction emulation faults".to_string(),
            detailed_description: "Instruction emulation faults. This counts instructions that had to be emulated by the kernel.".to_string(),
        },
        EventInfo {
            name: "dummy".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "Placeholder for collecting sample records".to_string(),
            detailed_description: "Placeholder event for collecting informational sample records without counting actual events.".to_string(),
        },
        EventInfo {
            name: "bpf-output".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "Streaming data from eBPF programs".to_string(),
            detailed_description: "Special event type for streaming data from eBPF programs. See bpf-helpers(7) for details.".to_string(),
        },
        EventInfo {
            name: "cgroup-switches".to_string(),
            aliases: vec![],
            category: "Software event".to_string(),
            description: "Context switches to a task in a different cgroup".to_string(),
            detailed_description: "Context switches to a task in a different cgroup. This counts switches between tasks in different cgroups.".to_string(),
        },
    ]
}

/// Get raw event format description
fn get_raw_event_info() -> EventInfo {
    EventInfo {
        name: "rNNNN".to_string(),
        aliases: vec!["RNNNN".to_string()],
        category: "Raw event".to_string(),
        description: "Raw CPU-specific event by hex config value".to_string(),
        detailed_description: "Raw CPU-specific event. NNNN is a hexadecimal config value (e.g., r1a8 for Intel RETIRED_UOP_TYPES). Use r0xNNNN format for 0x prefix. Consult CPU documentation for event codes.".to_string(),
    }
}

/// Get all tracepoint events from tracefs.
///
/// Returns tracepoints grouped by subsystem. Each tracepoint is represented
/// as `subsystem:event_name` format.
///
/// # Errors
///
/// Returns an empty vector if tracefs is not mounted or permission is denied,
/// allowing the list command to continue showing other events.
fn get_tracepoint_events() -> Vec<EventInfo> {
    match tracepoint::discover_tracepoints() {
        Ok(tracepoints) => tracepoints
            .into_iter()
            .map(|(subsystem, name)| EventInfo {
                name: format!("{}:{}", subsystem, name),
                aliases: vec![],
                category: "Tracepoint event".to_string(),
                description: format!("Tracepoint: {}.{}", subsystem, name),
                detailed_description: format!(
                    "Kernel tracepoint in the {} subsystem. Use 'perf record -e {}:{}' to record samples.",
                    subsystem, subsystem, name
                ),
            })
            .collect(),
        Err(_) => vec![],
    }
}

/// Format an event for display
fn format_event(event: &EventInfo, detailed: bool) -> String {
    let mut line = format!("  {:<40}", event.name);

    if !event.aliases.is_empty() {
        let aliases_str = event.aliases.join(" OR ");
        line = format!("  {:<20} OR {:<20}", event.name, aliases_str);
    }

    line.push_str(&format!("[{}]", event.category));

    let description = if detailed {
        &event.detailed_description
    } else {
        &event.description
    };
    line.push_str(&format!("\n    {}", description));

    line
}

/// Check if an event matches a filter pattern
fn matches_filter(event: &EventInfo, filter: &str) -> bool {
    let filter_lower = filter.to_lowercase();

    if event.name.to_lowercase().contains(&filter_lower) {
        return true;
    }

    for alias in &event.aliases {
        if alias.to_lowercase().contains(&filter_lower) {
            return true;
        }
    }

    if event.description.to_lowercase().contains(&filter_lower) {
        return true;
    }

    if event.category.to_lowercase().contains(&filter_lower) {
        return true;
    }

    false
}

/// Execute the list command
pub fn execute(filter: Option<&str>, detailed: bool, no_pager: bool) -> Result<()> {
    let mut events = Vec::new();

    events.extend(get_hardware_events());
    events.extend(get_software_events());
    events.push(get_raw_event_info());
    events.extend(get_tracepoint_events());

    let filtered_events: Vec<_> = if let Some(filter_str) = filter {
        events
            .into_iter()
            .filter(|e| matches_filter(e, filter_str))
            .collect()
    } else {
        events
    };

    // Determine if we should use a pager
    let pager = Pager::new();
    let use_pager = !no_pager && pager.should_use_pager();

    // Create output writer - either pager or stdout
    let mut output: Box<dyn Write> = if use_pager {
        pager.spawn().context("Failed to spawn pager")?
    } else {
        Box::new(std::io::stdout())
    };

    if filtered_events.is_empty() {
        writeln!(output, "No events found matching filter: {:?}", filter)?;
        return Ok(());
    }

    let mut hardware_events: Vec<_> = filtered_events
        .iter()
        .filter(|e| e.category == "Hardware event")
        .collect();
    let mut software_events: Vec<_> = filtered_events
        .iter()
        .filter(|e| e.category == "Software event")
        .collect();
    let raw_events: Vec<_> = filtered_events
        .iter()
        .filter(|e| e.category == "Raw event")
        .collect();
    let tracepoint_events: Vec<_> = filtered_events
        .iter()
        .filter(|e| e.category == "Tracepoint event")
        .collect();

    hardware_events.sort_by_key(|e| e.name.clone());
    software_events.sort_by_key(|e| e.name.clone());

    if !hardware_events.is_empty() {
        writeln!(output, "\nList of hardware events:")?;
        for event in hardware_events {
            writeln!(output, "{}", format_event(event, detailed))?;
        }
    }

    if !software_events.is_empty() {
        writeln!(output, "\nList of software events:")?;
        for event in software_events {
            writeln!(output, "{}", format_event(event, detailed))?;
        }
    }

    if !raw_events.is_empty() {
        writeln!(output, "\nList of raw events:")?;
        for event in raw_events {
            writeln!(output, "{}", format_event(event, detailed))?;
        }
    }

    if !tracepoint_events.is_empty() {
        writeln!(output, "\nList of tracepoint events:")?;
        display_tracepoints_by_subsystem(&mut output, &tracepoint_events, detailed)?;
    }

    // Flush output before pager is dropped (which waits for child to finish)
    output.flush().context("Failed to flush output")?;

    Ok(())
}

/// Display tracepoints grouped by subsystem.
fn display_tracepoints_by_subsystem(
    output: &mut Box<dyn Write>,
    tracepoints: &[&EventInfo],
    detailed: bool,
) -> Result<()> {
    let mut by_subsystem: BTreeMap<String, Vec<&EventInfo>> = BTreeMap::new();

    for tp in tracepoints {
        let subsystem = tp.name.split(':').next().unwrap_or("unknown").to_string();
        by_subsystem.entry(subsystem).or_default().push(*tp);
    }

    for (subsystem, mut events) in by_subsystem {
        events.sort_by_key(|e| e.name.clone());

        writeln!(output, "\n  {}:", subsystem)?;
        for event in events {
            let formatted = format_tracepoint_event(event, detailed);
            writeln!(output, "    {}", formatted)?;
        }
    }

    Ok(())
}

/// Format a tracepoint event for display (without subsystem prefix in name).
fn format_tracepoint_event(event: &EventInfo, detailed: bool) -> String {
    let name_without_subsystem = event
        .name
        .split_once(':')
        .map(|(_, name)| name)
        .unwrap_or(&event.name);

    let mut line = format!("{:<36}", name_without_subsystem);
    line.push_str("[Tracepoint event]");

    let description = if detailed {
        &event.detailed_description
    } else {
        &event.description
    };
    line.push_str(&format!("\n      {}", description));

    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hardware_events() {
        let events = get_hardware_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "cpu-cycles"));
        assert!(events.iter().any(|e| e.name == "instructions"));
    }

    #[test]
    fn test_get_software_events() {
        let events = get_software_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "cpu-clock"));
        assert!(events.iter().any(|e| e.name == "page-faults"));
    }

    #[test]
    fn test_get_raw_event_info() {
        let event = get_raw_event_info();
        assert_eq!(event.name, "rNNNN");
        assert!(event.aliases.contains(&"RNNNN".to_string()));
        assert_eq!(event.category, "Raw event");
    }

    #[test]
    fn test_matches_filter_name() {
        let event = EventInfo {
            name: "cpu-cycles".to_string(),
            aliases: vec!["cycles".to_string()],
            category: "Hardware event".to_string(),
            description: "Total cycles".to_string(),
            detailed_description: "Detailed description".to_string(),
        };

        assert!(matches_filter(&event, "cpu"));
        assert!(matches_filter(&event, "cycles"));
        assert!(matches_filter(&event, "CYCLES"));
        assert!(!matches_filter(&event, "cache"));
    }

    #[test]
    fn test_matches_filter_alias() {
        let event = EventInfo {
            name: "branch-instructions".to_string(),
            aliases: vec!["branches".to_string()],
            category: "Hardware event".to_string(),
            description: "Retired branch instructions".to_string(),
            detailed_description: "Detailed description".to_string(),
        };

        assert!(matches_filter(&event, "branch"));
        assert!(matches_filter(&event, "branches"));
        assert!(!matches_filter(&event, "cache"));
    }

    #[test]
    fn test_matches_filter_category() {
        let event = EventInfo {
            name: "cpu-cycles".to_string(),
            aliases: vec![],
            category: "Hardware event".to_string(),
            description: "Total cycles".to_string(),
            detailed_description: "Detailed description".to_string(),
        };

        assert!(matches_filter(&event, "hardware"));
        assert!(!matches_filter(&event, "software"));
    }

    #[test]
    fn test_format_event_simple() {
        let event = EventInfo {
            name: "cpu-cycles".to_string(),
            aliases: vec![],
            category: "Hardware event".to_string(),
            description: "Total cycles".to_string(),
            detailed_description: "Detailed description".to_string(),
        };

        let formatted = format_event(&event, false);
        assert!(formatted.contains("cpu-cycles"));
        assert!(formatted.contains("[Hardware event]"));
        assert!(formatted.contains("Total cycles"));
    }

    #[test]
    fn test_format_event_detailed() {
        let event = EventInfo {
            name: "cpu-cycles".to_string(),
            aliases: vec![],
            category: "Hardware event".to_string(),
            description: "Total cycles".to_string(),
            detailed_description: "Detailed description here".to_string(),
        };

        let formatted = format_event(&event, true);
        assert!(formatted.contains("Detailed description here"));
    }

    #[test]
    fn test_format_event_with_aliases() {
        let event = EventInfo {
            name: "cpu-cycles".to_string(),
            aliases: vec!["cycles".to_string()],
            category: "Hardware event".to_string(),
            description: "Total cycles".to_string(),
            detailed_description: "Detailed description".to_string(),
        };

        let formatted = format_event(&event, false);
        assert!(formatted.contains("cpu-cycles"));
        assert!(formatted.contains("cycles"));
        assert!(formatted.contains("OR"));
    }

    #[test]
    fn test_get_tracepoint_events_returns_vec() {
        let events = get_tracepoint_events();
        // Should return a vector (may be empty if tracefs not mounted)
        for event in &events {
            assert_eq!(event.category, "Tracepoint event");
            assert!(event.name.contains(':'));
        }
    }

    #[test]
    fn test_format_tracepoint_event() {
        let event = EventInfo {
            name: "sched:sched_switch".to_string(),
            aliases: vec![],
            category: "Tracepoint event".to_string(),
            description: "Tracepoint: sched.sched_switch".to_string(),
            detailed_description: "Detailed tracepoint info".to_string(),
        };

        let formatted = format_tracepoint_event(&event, false);
        assert!(formatted.contains("sched_switch"));
        assert!(!formatted.contains("sched:sched_switch"));
        assert!(formatted.contains("[Tracepoint event]"));
    }

    #[test]
    fn test_format_tracepoint_event_detailed() {
        let event = EventInfo {
            name: "syscalls:sys_enter_openat".to_string(),
            aliases: vec![],
            category: "Tracepoint event".to_string(),
            description: "Short desc".to_string(),
            detailed_description: "Detailed tracepoint description".to_string(),
        };

        let formatted = format_tracepoint_event(&event, true);
        assert!(formatted.contains("sys_enter_openat"));
        assert!(formatted.contains("Detailed tracepoint description"));
    }

    #[test]
    fn test_matches_filter_tracepoint_subsystem() {
        let event = EventInfo {
            name: "sched:sched_switch".to_string(),
            aliases: vec![],
            category: "Tracepoint event".to_string(),
            description: "Tracepoint: sched.sched_switch".to_string(),
            detailed_description: "Detailed".to_string(),
        };

        assert!(matches_filter(&event, "sched"));
        assert!(matches_filter(&event, "SCHED"));
        assert!(matches_filter(&event, "switch"));
        assert!(!matches_filter(&event, "syscall"));
    }

    #[test]
    fn test_matches_filter_tracepoint_full_name() {
        let event = EventInfo {
            name: "syscalls:sys_enter_openat".to_string(),
            aliases: vec![],
            category: "Tracepoint event".to_string(),
            description: "Tracepoint: syscalls.sys_enter_openat".to_string(),
            detailed_description: "Detailed".to_string(),
        };

        assert!(matches_filter(&event, "syscalls"));
        assert!(matches_filter(&event, "openat"));
        assert!(matches_filter(&event, "sys_enter"));
        assert!(!matches_filter(&event, "sched"));
    }

    #[test]
    fn test_matches_filter_tracepoint_category() {
        let event = EventInfo {
            name: "irq:irq_handler_entry".to_string(),
            aliases: vec![],
            category: "Tracepoint event".to_string(),
            description: "Tracepoint: irq.irq_handler_entry".to_string(),
            detailed_description: "Detailed".to_string(),
        };

        assert!(matches_filter(&event, "tracepoint"));
        assert!(!matches_filter(&event, "hardware"));
    }
}
