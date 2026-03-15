//! Shared event parsing for all perf-rs commands.
//!
//! This module provides a unified event type and parser that can be used
//! across stat, record, and other commands.

use anyhow::{anyhow, Result};
use perf_event::events::Event;

// Re-export event types from perf_event crate
pub use perf_event::events::{Cache, CacheId, CacheOp, CacheResult, Hardware, Raw, Software};

/// A unified event type that can be hardware, software, cache, or raw events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerfEvent {
    Hardware(Hardware),
    Software(Software),
    Cache(Cache),
    Raw(Raw),
}

impl Event for PerfEvent {
    fn update_attrs(self, attr: &mut perf_event::hooks::sys::bindings::perf_event_attr) {
        match self {
            PerfEvent::Hardware(h) => h.update_attrs(attr),
            PerfEvent::Software(s) => s.update_attrs(attr),
            PerfEvent::Cache(c) => c.update_attrs(attr),
            PerfEvent::Raw(r) => r.update_attrs(attr),
        }
    }
}

impl PerfEvent {
    pub fn is_hardware(&self) -> bool {
        matches!(self, PerfEvent::Hardware(_))
    }

    pub fn is_software(&self) -> bool {
        matches!(self, PerfEvent::Software(_))
    }

    pub fn is_cache(&self) -> bool {
        matches!(self, PerfEvent::Cache(_))
    }

    pub fn is_raw(&self) -> bool {
        matches!(self, PerfEvent::Raw(_))
    }
}

/// Parse an event name string to a PerfEvent.
///
/// Supports:
/// - Hardware events (cpu-cycles, instructions, cache-references, etc.)
/// - Software events (cpu-clock, task-clock, page-faults, etc.)
/// - Cache events (L1-dcache-loads, L1-dcache-misses, etc.)
/// - Raw events (rNNNN where NNNN is a hex config value)
///
/// # Errors
///
/// Returns an error if the event name is not recognized.
pub fn parse_event(name: &str) -> Result<PerfEvent> {
    let name = name.trim();

    // Try raw events first (rNNNN format)
    if let Some(event) = parse_raw_event(name) {
        return Ok(event);
    }

    let name_lower = name.to_lowercase();

    // Try hardware events
    if let Some(event) = parse_hardware_event(&name_lower) {
        return Ok(event);
    }

    // Try software events
    if let Some(event) = parse_software_event(&name_lower) {
        return Ok(event);
    }

    // Try cache events
    if let Some(event) = parse_cache_event(&name_lower) {
        return Ok(event);
    }

    Err(anyhow!(
        "Unknown event: '{}'. Run 'perf list' to see available events.",
        name
    ))
}

/// Parse a raw event in rNNNN format.
///
/// Format: `rNNNN` where NNNN is a hexadecimal config value.
/// Examples: `r1a8`, `r00c0`, `r0x1a8` (0x prefix optional)
fn parse_raw_event(name: &str) -> Option<PerfEvent> {
    let name = name.trim();

    let hex_str = if let Some(rest) = name.strip_prefix('r') {
        rest
    } else if let Some(rest) = name.strip_prefix('R') {
        rest
    } else {
        return None;
    };

    // Allow optional 0x prefix
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);

    if let Ok(config) = u64::from_str_radix(hex_str, 16) {
        return Some(PerfEvent::Raw(Raw::new(config)));
    }

    None
}

/// Parse a hardware event name.
fn parse_hardware_event(name: &str) -> Option<PerfEvent> {
    let hardware = match name {
        "cpu-cycles" | "cycles" => Hardware::CPU_CYCLES,
        "instructions" | "instructions-retired" => Hardware::INSTRUCTIONS,
        "cache-references" => Hardware::CACHE_REFERENCES,
        "cache-misses" => Hardware::CACHE_MISSES,
        "branch-instructions" | "branches" => Hardware::BRANCH_INSTRUCTIONS,
        "branch-misses" => Hardware::BRANCH_MISSES,
        "bus-cycles" => Hardware::BUS_CYCLES,
        "stalled-cycles-frontend" | "idle-cycles-frontend" => Hardware::STALLED_CYCLES_FRONTEND,
        "stalled-cycles-backend" | "idle-cycles-backend" => Hardware::STALLED_CYCLES_BACKEND,
        "ref-cpu-cycles" | "ref-cycles" | "cpu-cycles-ref" => Hardware::REF_CPU_CYCLES,
        _ => return None,
    };
    Some(PerfEvent::Hardware(hardware))
}

/// Parse a software event name.
fn parse_software_event(name: &str) -> Option<PerfEvent> {
    let software = match name {
        "cpu-clock" => Software::CPU_CLOCK,
        "task-clock" => Software::TASK_CLOCK,
        "page-faults" | "faults" => Software::PAGE_FAULTS,
        "context-switches" | "cs" => Software::CONTEXT_SWITCHES,
        "cpu-migrations" => Software::CPU_MIGRATIONS,
        "minor-faults" => Software::PAGE_FAULTS_MIN,
        "major-faults" => Software::PAGE_FAULTS_MAJ,
        "alignment-faults" => Software::ALIGNMENT_FAULTS,
        "emulation-faults" => Software::EMULATION_FAULTS,
        "dummy" => Software::DUMMY,
        "bpf-output" => Software::BPF_OUTPUT,
        "cgroup-switches" => Software::CGROUP_SWITCHES,
        _ => return None,
    };
    Some(PerfEvent::Software(software))
}

/// Parse a cache event name.
///
/// Format: `[cache-level]-[cache-type]-[operation]-[result]`
/// Examples: `L1-dcache-loads`, `L1-dcache-load-misses`, `LLC-loads`, `dTLB-load-misses`
fn parse_cache_event(name: &str) -> Option<PerfEvent> {
    // Parse cache event name format
    // Common formats:
    // - L1-dcache-loads, L1-dcache-stores, L1-dcache-prefetches
    // - L1-dcache-load-misses, L1-dcache-store-misses
    // - L1-icache-loads, L1-icache-load-misses
    // - LLC-loads, LLC-stores, LLC-load-misses
    // - dTLB-loads, dTLB-load-misses, iTLB-loads, iTLB-load-misses
    // - branch-loads, branch-load-misses

    let (cache_id, remainder) = parse_cache_id(name)?;
    let (operation, result) = parse_cache_op_and_result(remainder)?;

    Some(PerfEvent::Cache(Cache {
        which: cache_id,
        operation,
        result,
    }))
}

/// Parse the cache ID from the beginning of the event name.
fn parse_cache_id(name: &str) -> Option<(CacheId, &str)> {
    // L1 data cache
    if let Some(rest) = name.strip_prefix("L1-dcache-") {
        return Some((CacheId::L1D, rest));
    }
    if let Some(rest) = name.strip_prefix("l1-dcache-") {
        return Some((CacheId::L1D, rest));
    }
    if let Some(rest) = name.strip_prefix("L1-d-") {
        return Some((CacheId::L1D, rest));
    }
    if let Some(rest) = name.strip_prefix("l1-d-") {
        return Some((CacheId::L1D, rest));
    }

    // L1 instruction cache
    if let Some(rest) = name.strip_prefix("L1-icache-") {
        return Some((CacheId::L1I, rest));
    }
    if let Some(rest) = name.strip_prefix("l1-icache-") {
        return Some((CacheId::L1I, rest));
    }
    if let Some(rest) = name.strip_prefix("L1-i-") {
        return Some((CacheId::L1I, rest));
    }
    if let Some(rest) = name.strip_prefix("l1-i-") {
        return Some((CacheId::L1I, rest));
    }

    // Last-level cache
    if let Some(rest) = name.strip_prefix("LLC-") {
        return Some((CacheId::LL, rest));
    }
    if let Some(rest) = name.strip_prefix("llc-") {
        return Some((CacheId::LL, rest));
    }

    // Data TLB
    if let Some(rest) = name.strip_prefix("dTLB-") {
        return Some((CacheId::DTLB, rest));
    }
    if let Some(rest) = name.strip_prefix("dtlb-") {
        return Some((CacheId::DTLB, rest));
    }

    // Instruction TLB
    if let Some(rest) = name.strip_prefix("iTLB-") {
        return Some((CacheId::ITLB, rest));
    }
    if let Some(rest) = name.strip_prefix("itlb-") {
        return Some((CacheId::ITLB, rest));
    }

    // Branch prediction unit
    if let Some(rest) = name.strip_prefix("branch-") {
        return Some((CacheId::BPU, rest));
    }

    // Node (NUMA)
    if let Some(rest) = name.strip_prefix("node-") {
        return Some((CacheId::NODE, rest));
    }

    None
}

/// Parse the cache operation and result from the remainder of the event name.
fn parse_cache_op_and_result(remainder: &str) -> Option<(CacheOp, CacheResult)> {
    // Parse operation and result
    // Formats: loads, stores, prefetches, load-misses, store-misses, prefetch-misses

    match remainder {
        "loads" | "read-accesses" | "reads" => Some((CacheOp::READ, CacheResult::ACCESS)),
        "load-misses" | "read-misses" => Some((CacheOp::READ, CacheResult::MISS)),
        "stores" | "write-accesses" | "writes" => Some((CacheOp::WRITE, CacheResult::ACCESS)),
        "store-misses" | "write-misses" => Some((CacheOp::WRITE, CacheResult::MISS)),
        "prefetches" | "prefetch-accesses" => Some((CacheOp::PREFETCH, CacheResult::ACCESS)),
        "prefetch-misses" => Some((CacheOp::PREFETCH, CacheResult::MISS)),
        "accesses" => Some((CacheOp::READ, CacheResult::ACCESS)),
        "misses" => Some((CacheOp::READ, CacheResult::MISS)),
        _ => None,
    }
}

/// Format a PerfEvent to a human-readable name.
pub fn format_event_name(event: &PerfEvent) -> String {
    match event {
        PerfEvent::Hardware(h) => format_hardware_name(*h),
        PerfEvent::Software(s) => format_software_name(*s),
        PerfEvent::Cache(c) => format_cache_name(c.clone()),
        PerfEvent::Raw(r) => format!("r{:x}", r.config),
    }
}

fn format_hardware_name(event: Hardware) -> String {
    match event {
        Hardware::CPU_CYCLES => "cpu-cycles".to_string(),
        Hardware::INSTRUCTIONS => "instructions".to_string(),
        Hardware::CACHE_REFERENCES => "cache-references".to_string(),
        Hardware::CACHE_MISSES => "cache-misses".to_string(),
        Hardware::BRANCH_INSTRUCTIONS => "branch-instructions".to_string(),
        Hardware::BRANCH_MISSES => "branch-misses".to_string(),
        Hardware::BUS_CYCLES => "bus-cycles".to_string(),
        Hardware::STALLED_CYCLES_FRONTEND => "stalled-cycles-frontend".to_string(),
        Hardware::STALLED_CYCLES_BACKEND => "stalled-cycles-backend".to_string(),
        Hardware::REF_CPU_CYCLES => "ref-cycles".to_string(),
        _ => format!("hardware-{:?}", event),
    }
}

fn format_software_name(event: Software) -> String {
    match event {
        Software::CPU_CLOCK => "cpu-clock".to_string(),
        Software::TASK_CLOCK => "task-clock".to_string(),
        Software::PAGE_FAULTS => "page-faults".to_string(),
        Software::CONTEXT_SWITCHES => "context-switches".to_string(),
        Software::CPU_MIGRATIONS => "cpu-migrations".to_string(),
        Software::PAGE_FAULTS_MIN => "minor-faults".to_string(),
        Software::PAGE_FAULTS_MAJ => "major-faults".to_string(),
        Software::ALIGNMENT_FAULTS => "alignment-faults".to_string(),
        Software::EMULATION_FAULTS => "emulation-faults".to_string(),
        Software::DUMMY => "dummy".to_string(),
        Software::BPF_OUTPUT => "bpf-output".to_string(),
        Software::CGROUP_SWITCHES => "cgroup-switches".to_string(),
        _ => format!("software-{:?}", event),
    }
}

fn format_cache_name(cache: Cache) -> String {
    let cache_name = match cache.which {
        CacheId::L1D => "L1-dcache",
        CacheId::L1I => "L1-icache",
        CacheId::LL => "LLC",
        CacheId::DTLB => "dTLB",
        CacheId::ITLB => "iTLB",
        CacheId::BPU => "branch",
        CacheId::NODE => "node",
        _ => "cache",
    };

    let op_name = match cache.operation {
        CacheOp::READ => "load",
        CacheOp::WRITE => "store",
        CacheOp::PREFETCH => "prefetch",
        _ => "op",
    };

    let result_suffix = match cache.result {
        CacheResult::ACCESS => "s",
        CacheResult::MISS => "-misses",
        _ => "",
    };

    format!("{}-{}{}", cache_name, op_name, result_suffix)
}

/// Parse a comma-separated event string into a vector of PerfEvents.
pub fn parse_events(events_str: &str) -> Result<Vec<PerfEvent>> {
    events_str
        .split(',')
        .map(|s| parse_event(s.trim()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hardware_events() {
        assert!(matches!(
            parse_event("cpu-cycles"),
            Ok(PerfEvent::Hardware(Hardware::CPU_CYCLES))
        ));
        assert!(matches!(
            parse_event("cycles"),
            Ok(PerfEvent::Hardware(Hardware::CPU_CYCLES))
        ));
        assert!(matches!(
            parse_event("instructions"),
            Ok(PerfEvent::Hardware(Hardware::INSTRUCTIONS))
        ));
        assert!(matches!(
            parse_event("cache-misses"),
            Ok(PerfEvent::Hardware(Hardware::CACHE_MISSES))
        ));
        assert!(matches!(
            parse_event("branch-misses"),
            Ok(PerfEvent::Hardware(Hardware::BRANCH_MISSES))
        ));
        assert!(matches!(
            parse_event("bus-cycles"),
            Ok(PerfEvent::Hardware(Hardware::BUS_CYCLES))
        ));
        assert!(matches!(
            parse_event("ref-cycles"),
            Ok(PerfEvent::Hardware(Hardware::REF_CPU_CYCLES))
        ));
    }

    #[test]
    fn test_parse_cache_events() {
        assert!(matches!(
            parse_event("L1-dcache-loads"),
            Ok(PerfEvent::Cache(Cache {
                which: CacheId::L1D,
                operation: CacheOp::READ,
                result: CacheResult::ACCESS,
            }))
        ));
        assert!(matches!(
            parse_event("L1-dcache-load-misses"),
            Ok(PerfEvent::Cache(Cache {
                which: CacheId::L1D,
                operation: CacheOp::READ,
                result: CacheResult::MISS,
            }))
        ));
        assert!(matches!(
            parse_event("LLC-loads"),
            Ok(PerfEvent::Cache(Cache {
                which: CacheId::LL,
                operation: CacheOp::READ,
                result: CacheResult::ACCESS,
            }))
        ));
        assert!(matches!(
            parse_event("dTLB-load-misses"),
            Ok(PerfEvent::Cache(Cache {
                which: CacheId::DTLB,
                operation: CacheOp::READ,
                result: CacheResult::MISS,
            }))
        ));
    }

    #[test]
    fn test_parse_raw_events() {
        assert!(matches!(
            parse_event("r1a8"),
            Ok(PerfEvent::Raw(Raw { config: 0x1a8, .. }))
        ));
        assert!(matches!(
            parse_event("r00c0"),
            Ok(PerfEvent::Raw(Raw { config: 0xc0, .. }))
        ));
        assert!(matches!(
            parse_event("r0x1a8"),
            Ok(PerfEvent::Raw(Raw { config: 0x1a8, .. }))
        ));
        assert!(matches!(
            parse_event("R1A8"),
            Ok(PerfEvent::Raw(Raw { config: 0x1a8, .. }))
        ));
    }

    #[test]
    fn test_parse_events_comma_separated() {
        let events = parse_events("cpu-cycles,instructions,cache-misses").unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[0],
            PerfEvent::Hardware(Hardware::CPU_CYCLES)
        ));
        assert!(matches!(
            events[1],
            PerfEvent::Hardware(Hardware::INSTRUCTIONS)
        ));
        assert!(matches!(
            events[2],
            PerfEvent::Hardware(Hardware::CACHE_MISSES)
        ));
    }

    #[test]
    fn test_parse_event_unknown() {
        assert!(parse_event("unknown-event").is_err());
    }

    #[test]
    fn test_format_event_name() {
        assert_eq!(
            format_event_name(&PerfEvent::Hardware(Hardware::CPU_CYCLES)),
            "cpu-cycles"
        );
        assert_eq!(
            format_event_name(&PerfEvent::Software(Software::CPU_CLOCK)),
            "cpu-clock"
        );
        assert_eq!(
            format_event_name(&PerfEvent::Cache(Cache {
                which: CacheId::L1D,
                operation: CacheOp::READ,
                result: CacheResult::ACCESS,
            })),
            "L1-dcache-loads"
        );
        assert_eq!(format_event_name(&PerfEvent::Raw(Raw::new(0x1a8))), "r1a8");
    }
}
