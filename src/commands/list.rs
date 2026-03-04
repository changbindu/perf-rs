//! List available performance events
//!
//! This module implements the `perf list` command which displays available
//! hardware and software performance events.

/// Event information for display
struct EventInfo {
    /// Primary name of the event
    name: &'static str,
    /// Alternative names (aliases)
    aliases: Vec<&'static str>,
    /// Event category (Hardware, Software, etc.)
    category: &'static str,
    /// Brief description
    description: &'static str,
    /// Detailed description (shown with --detailed flag)
    detailed_description: &'static str,
}

/// Get all hardware events
fn get_hardware_events() -> Vec<EventInfo> {
    vec![
        EventInfo {
            name: "cpu-cycles",
            aliases: vec!["cycles"],
            category: "Hardware event",
            description: "Total cycles",
            detailed_description: "Total cycles. Be aware of frequency scaling and turbo mode affecting this count.",
        },
        EventInfo {
            name: "instructions",
            aliases: vec![],
            category: "Hardware event",
            description: "Retired instructions",
            detailed_description: "Retired instructions. This counts the number of instructions that have completed execution.",
        },
        EventInfo {
            name: "cache-references",
            aliases: vec![],
            category: "Hardware event",
            description: "Cache accesses",
            detailed_description: "Cache accesses. This counts all cache accesses, both hits and misses.",
        },
        EventInfo {
            name: "cache-misses",
            aliases: vec![],
            category: "Hardware event",
            description: "Cache misses",
            detailed_description: "Cache misses. This counts cache accesses that missed the cache.",
        },
        EventInfo {
            name: "branch-instructions",
            aliases: vec!["branches"],
            category: "Hardware event",
            description: "Retired branch instructions",
            detailed_description: "Retired branch instructions. This counts all branch instructions that were executed.",
        },
        EventInfo {
            name: "branch-misses",
            aliases: vec![],
            category: "Hardware event",
            description: "Mispredicted branch instructions",
            detailed_description: "Mispredicted branch instructions. This counts branches that were incorrectly predicted by the CPU's branch predictor.",
        },
        EventInfo {
            name: "bus-cycles",
            aliases: vec![],
            category: "Hardware event",
            description: "Bus cycles",
            detailed_description: "Bus cycles. This counts cycles on the system bus.",
        },
        EventInfo {
            name: "stalled-cycles-frontend",
            aliases: vec!["idle-cycles-frontend"],
            category: "Hardware event",
            description: "Stalled cycles during issue",
            detailed_description: "Stalled cycles during issue. This counts cycles where the CPU frontend (instruction fetch/decode) is stalled.",
        },
        EventInfo {
            name: "stalled-cycles-backend",
            aliases: vec!["idle-cycles-backend"],
            category: "Hardware event",
            description: "Stalled cycles during retirement",
            detailed_description: "Stalled cycles during retirement. This counts cycles where the CPU backend (execution units) is stalled.",
        },
        EventInfo {
            name: "ref-cycles",
            aliases: vec![],
            category: "Hardware event",
            description: "Total cycles (independent of frequency scaling)",
            detailed_description: "Total cycles, independent of frequency scaling. This uses a fixed-frequency reference clock.",
        },
    ]
}

/// Get all software events
fn get_software_events() -> Vec<EventInfo> {
    vec![
        EventInfo {
            name: "cpu-clock",
            aliases: vec![],
            category: "Software event",
            description: "High-resolution per-CPU timer",
            detailed_description: "High-resolution per-CPU timer. This measures CPU time using the CPU's high-resolution timer.",
        },
        EventInfo {
            name: "task-clock",
            aliases: vec![],
            category: "Software event",
            description: "Per-task clock count",
            detailed_description: "Per-task clock count. This measures the time a task is running on a CPU.",
        },
        EventInfo {
            name: "page-faults",
            aliases: vec!["faults"],
            category: "Software event",
            description: "Page faults",
            detailed_description: "Page faults. This counts both minor and major page faults.",
        },
        EventInfo {
            name: "context-switches",
            aliases: vec!["cs"],
            category: "Software event",
            description: "Context switches",
            detailed_description: "Context switches. This counts the number of times the CPU switched from one task to another.",
        },
        EventInfo {
            name: "cpu-migrations",
            aliases: vec![],
            category: "Software event",
            description: "Process migration to another CPU",
            detailed_description: "Process migration to another CPU. This counts when a process moves from one CPU to another.",
        },
        EventInfo {
            name: "minor-faults",
            aliases: vec![],
            category: "Software event",
            description: "Minor page faults (resolved without I/O)",
            detailed_description: "Minor page faults. These are page faults that can be resolved without disk I/O, typically by mapping an existing page in memory.",
        },
        EventInfo {
            name: "major-faults",
            aliases: vec![],
            category: "Software event",
            description: "Major page faults (I/O required)",
            detailed_description: "Major page faults. These are page faults that require disk I/O to resolve, typically loading a page from swap or a file.",
        },
        EventInfo {
            name: "alignment-faults",
            aliases: vec![],
            category: "Software event",
            description: "Alignment faults (kernel intervention required)",
            detailed_description: "Alignment faults that required kernel intervention. Note: This is only generated on some CPUs, never on x86_64 or ARM.",
        },
        EventInfo {
            name: "emulation-faults",
            aliases: vec![],
            category: "Software event",
            description: "Instruction emulation faults",
            detailed_description: "Instruction emulation faults. This counts instructions that had to be emulated by the kernel.",
        },
        EventInfo {
            name: "dummy",
            aliases: vec![],
            category: "Software event",
            description: "Placeholder for collecting sample records",
            detailed_description: "Placeholder event for collecting informational sample records without counting actual events.",
        },
        EventInfo {
            name: "bpf-output",
            aliases: vec![],
            category: "Software event",
            description: "Streaming data from eBPF programs",
            detailed_description: "Special event type for streaming data from eBPF programs. See bpf-helpers(7) for details.",
        },
        EventInfo {
            name: "cgroup-switches",
            aliases: vec![],
            category: "Software event",
            description: "Context switches to a task in a different cgroup",
            detailed_description: "Context switches to a task in a different cgroup. This counts switches between tasks in different cgroups.",
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
        event.detailed_description
    } else {
        event.description
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
pub fn execute(filter: Option<&str>, detailed: bool) -> crate::error::Result<()> {
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

    if filtered_events.is_empty() {
        println!("No events found matching filter: {:?}", filter);
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

    hardware_events.sort_by_key(|e| e.name);
    software_events.sort_by_key(|e| e.name);

    if !hardware_events.is_empty() {
        println!("\nList of hardware events:");
        for event in hardware_events {
            println!("{}", format_event(event, detailed));
        }
    }

    if !software_events.is_empty() {
        println!("\nList of software events:");
        for event in software_events {
            println!("{}", format_event(event, detailed));
        }
    }

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
            name: "cpu-cycles",
            aliases: vec!["cycles"],
            category: "Hardware event",
            description: "Total cycles",
            detailed_description: "Detailed description",
        };

        assert!(matches_filter(&event, "cpu"));
        assert!(matches_filter(&event, "cycles"));
        assert!(matches_filter(&event, "CYCLES"));
        assert!(!matches_filter(&event, "cache"));
    }

    #[test]
    fn test_matches_filter_alias() {
        let event = EventInfo {
            name: "branch-instructions",
            aliases: vec!["branches"],
            category: "Hardware event",
            description: "Retired branch instructions",
            detailed_description: "Detailed description",
        };

        assert!(matches_filter(&event, "branch"));
        assert!(matches_filter(&event, "branches"));
        assert!(!matches_filter(&event, "cache"));
    }

    #[test]
    fn test_matches_filter_category() {
        let event = EventInfo {
            name: "cpu-cycles",
            aliases: vec![],
            category: "Hardware event",
            description: "Total cycles",
            detailed_description: "Detailed description",
        };

        assert!(matches_filter(&event, "hardware"));
        assert!(!matches_filter(&event, "software"));
    }

    #[test]
    fn test_format_event_simple() {
        let event = EventInfo {
            name: "cpu-cycles",
            aliases: vec![],
            category: "Hardware event",
            description: "Total cycles",
            detailed_description: "Detailed description",
        };

        let formatted = format_event(&event, false);
        assert!(formatted.contains("cpu-cycles"));
        assert!(formatted.contains("[Hardware event]"));
        assert!(formatted.contains("Total cycles"));
    }

    #[test]
    fn test_format_event_detailed() {
        let event = EventInfo {
            name: "cpu-cycles",
            aliases: vec![],
            category: "Hardware event",
            description: "Total cycles",
            detailed_description: "Detailed description here",
        };

        let formatted = format_event(&event, true);
        assert!(formatted.contains("Detailed description here"));
    }

    #[test]
    fn test_format_event_with_aliases() {
        let event = EventInfo {
            name: "cpu-cycles",
            aliases: vec!["cycles"],
            category: "Hardware event",
            description: "Total cycles",
            detailed_description: "Detailed description",
        };

        let formatted = format_event(&event, false);
        assert!(formatted.contains("cpu-cycles"));
        assert!(formatted.contains("cycles"));
        assert!(formatted.contains("OR"));
    }
}
