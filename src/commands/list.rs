//! List available performance events
//!
//! This module implements the `perf list` command which displays available
//! hardware and software performance events.

use std::io::Write;

use anyhow::{Context, Result};

use crate::arch;
use crate::pager::Pager;

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

    // Flush output before pager is dropped (which waits for child to finish)
    output.flush().context("Failed to flush output")?;

    Ok(())
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
}
