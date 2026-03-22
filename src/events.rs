//! Shared event parsing for all perf-rs commands.
//!
//! This module provides a unified event type and parser that can be used
//! across stat, record, and other commands.
//!
//! # Event Modifiers
//!
//! Events can have modifiers appended with `:` to control privilege levels:
//! - `:u` - count in user space only (exclude kernel)
//! - `:k` - count in kernel space only (exclude user)
//! - `:h` - count in hypervisor only (exclude user and kernel)
//! - `:p` - use precise sampling (PEBS on Intel)
//!
//! Multiple modifiers can be combined: `cpu-cycles:uk` (both user and kernel)

use anyhow::{anyhow, Result};
use perf_event::events::Event;

// Re-export event types from perf_event crate
pub use perf_event::events::{Cache, CacheId, CacheOp, CacheResult, Hardware, Raw, Software};

use crate::tracepoint::TracepointId;

/// Event modifiers that control privilege level filtering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventModifiers {
    /// Exclude user-space events (only count kernel)
    pub exclude_user: bool,
    /// Exclude kernel-space events (only count user)
    pub exclude_kernel: bool,
    /// Exclude hypervisor events
    pub exclude_hv: bool,
    /// Use precise sampling (PEBS on Intel)
    pub precise: bool,
}

impl EventModifiers {
    /// Create a new empty modifiers struct (all fields false).
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse modifiers from a string like "uk", "uhp", etc.
    ///
    /// Returns the parsed modifiers and any unrecognized characters.
    pub fn parse(s: &str) -> (Self, String) {
        let mut mods = Self::new();
        let mut unknown = String::new();

        for c in s.chars() {
            match c {
                'u' => mods.exclude_kernel = true,
                'k' => mods.exclude_user = true,
                'h' => {
                    mods.exclude_user = true;
                    mods.exclude_kernel = true;
                }
                'p' => mods.precise = true,
                _ => unknown.push(c),
            }
        }

        (mods, unknown)
    }

    /// Check if any modifiers are set.
    pub fn is_empty(&self) -> bool {
        !self.exclude_user && !self.exclude_kernel && !self.exclude_hv && !self.precise
    }

    /// Get a string representation of the modifiers.
    pub fn to_suffix(&self) -> String {
        let mut s = String::new();
        if self.exclude_kernel && !self.exclude_user {
            s.push('u');
        }
        if self.exclude_user && !self.exclude_kernel {
            s.push('k');
        }
        if self.exclude_user && self.exclude_kernel {
            s.push('h');
        }
        if self.precise {
            s.push('p');
        }
        if !s.is_empty() {
            format!(":{}", s)
        } else {
            String::new()
        }
    }
}

/// A parsed event with optional modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEvent {
    /// The underlying performance event
    pub event: PerfEvent,
    /// Modifiers for this event
    pub modifiers: EventModifiers,
}

impl ParsedEvent {
    /// Create a new parsed event with no modifiers.
    pub fn new(event: PerfEvent) -> Self {
        Self {
            event,
            modifiers: EventModifiers::new(),
        }
    }

    /// Create a parsed event with specific modifiers.
    pub fn with_modifiers(event: PerfEvent, modifiers: EventModifiers) -> Self {
        Self { event, modifiers }
    }
}

/// A unified event type that can be hardware, software, cache, raw, or tracepoint events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerfEvent {
    Hardware(Hardware),
    Software(Software),
    Cache(Cache),
    Raw(Raw),
    Tracepoint(TracepointId),
}

impl Event for PerfEvent {
    fn update_attrs(self, attr: &mut perf_event::hooks::sys::bindings::perf_event_attr) {
        match self {
            PerfEvent::Hardware(h) => h.update_attrs(attr),
            PerfEvent::Software(s) => s.update_attrs(attr),
            PerfEvent::Cache(c) => c.update_attrs(attr),
            PerfEvent::Raw(r) => r.update_attrs(attr),
            PerfEvent::Tracepoint(t) => {
                perf_event::events::Tracepoint::with_id(t.id).update_attrs(attr)
            }
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

    pub fn is_tracepoint(&self) -> bool {
        matches!(self, PerfEvent::Tracepoint(_))
    }
}

/// Parse an event name string to a ParsedEvent with modifiers.
///
/// Supports:
/// - Hardware events (cpu-cycles, instructions, cache-references, etc.)
/// - Software events (cpu-clock, task-clock, page-faults, etc.)
/// - Cache events (L1-dcache-loads, L1-dcache-misses, etc.)
/// - Raw events (rNNNN where NNNN is a hex config value)
/// - Tracepoint events (subsystem:name format, e.g., sched:sched_switch)
/// - Event modifiers (:u, :k, :h, :p)
///
/// # Modifiers
///
/// - `:u` - user space only (exclude kernel)
/// - `:k` - kernel space only (exclude user)  
/// - `:h` - hypervisor only (exclude user and kernel)
/// - `:p` - precise sampling
///
/// # Examples
///
/// ```
/// # use perf_rs::events::{parse_event, EventModifiers};
/// let evt = parse_event("cpu-cycles:u").unwrap();
/// assert!(evt.modifiers.exclude_kernel);
/// assert!(!evt.modifiers.exclude_user);
/// ```
///
/// # Errors
///
/// Returns an error if the event name is not recognized.
pub fn parse_event(name: &str) -> Result<ParsedEvent> {
    let name = name.trim();

    // Try tracepoint parsing first if it looks like a tracepoint (subsystem:name)
    // Tracepoints have format "subsystem:name" where name contains underscores/letters
    // Modifiers are short sequences of: u, k, h, p
    if let Some(colon_pos) = name.find(':') {
        let after_colon = &name[colon_pos + 1..];

        // If it looks like a tracepoint name (contains letters, underscores, digits but NOT just modifier chars)
        // or has more than one colon, try tracepoint parsing first
        let is_likely_tracepoint = after_colon.contains('_')
            || after_colon.len() > 4
            || after_colon.contains(':')
            || !after_colon
                .chars()
                .all(|c| matches!(c, 'u' | 'k' | 'h' | 'p'));

        if is_likely_tracepoint {
            // Try tracepoint parsing
            if let Ok(event) = parse_event_base(name) {
                return Ok(ParsedEvent::new(event));
            }
        }
    }

    // Split event name and modifiers
    let (event_name, modifiers) = if let Some(colon_pos) = name.find(':') {
        let (base, mod_str) = name.split_at(colon_pos);
        let mod_str = &mod_str[1..];
        let (mods, unknown) = EventModifiers::parse(mod_str);
        if !unknown.is_empty() {
            // Fall back to trying tracepoint parsing
            if let Ok(event) = parse_event_base(name) {
                return Ok(ParsedEvent::new(event));
            }
            return Err(anyhow!(
                "Unknown event modifiers: '{}' in '{}'",
                unknown,
                name
            ));
        }
        (base, mods)
    } else {
        (name, EventModifiers::new())
    };

    let event = parse_event_base(event_name)?;

    Ok(ParsedEvent::with_modifiers(event, modifiers))
}

/// Parse the base event name without modifiers.
fn parse_event_base(name: &str) -> Result<PerfEvent> {
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

    // Try tracepoint events (subsystem:name format)
    if let Some(event) = parse_tracepoint_event(name)? {
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

/// Parse a tracepoint event in subsystem:name format.
///
/// Format: `subsystem:name` where subsystem is the tracepoint category
/// (e.g., "sched", "syscalls", "irq") and name is the specific tracepoint.
///
/// # Examples
/// - `sched:sched_switch`
/// - `syscalls:sys_enter_openat`
/// - `irq:irq_handler_entry`
fn parse_tracepoint_event(name: &str) -> Result<Option<PerfEvent>> {
    if !name.contains(':') {
        return Ok(None);
    }

    let parts: Vec<&str> = name.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Ok(None);
    }

    let subsystem = parts[0];
    let tp_name = parts[1];

    if subsystem.is_empty() || tp_name.is_empty() {
        return Ok(None);
    }

    match TracepointId::from_name(subsystem, tp_name) {
        Ok(tp) => Ok(Some(PerfEvent::Tracepoint(tp))),
        Err(e) => Err(anyhow::Error::msg(e.to_string())),
    }
}

/// Format a PerfEvent to a human-readable name.
pub fn format_event_name(event: &PerfEvent) -> String {
    match event {
        PerfEvent::Hardware(h) => format_hardware_name(*h),
        PerfEvent::Software(s) => format_software_name(*s),
        PerfEvent::Cache(c) => format_cache_name(c.clone()),
        PerfEvent::Raw(r) => format!("r{:x}", r.config),
        PerfEvent::Tracepoint(t) => t.full_name(),
    }
}

/// Format a ParsedEvent to a human-readable name including modifiers.
pub fn format_parsed_event_name(parsed: &ParsedEvent) -> String {
    let base = format_event_name(&parsed.event);
    let suffix = parsed.modifiers.to_suffix();
    format!("{}{}", base, suffix)
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

/// Parse a comma-separated event string into a vector of ParsedEvents.
pub fn parse_events(events_str: &str) -> Result<Vec<ParsedEvent>> {
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
        let evt = parse_event("cpu-cycles").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::CPU_CYCLES)
        ));
        assert!(evt.modifiers.is_empty());

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

        let evt = parse_event("branch-misses").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::BRANCH_MISSES)
        ));

        let evt = parse_event("bus-cycles").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::BUS_CYCLES)
        ));

        let evt = parse_event("ref-cycles").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::REF_CPU_CYCLES)
        ));
    }

    #[test]
    fn test_parse_cache_events() {
        let evt = parse_event("L1-dcache-loads").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Cache(Cache {
                which: CacheId::L1D,
                operation: CacheOp::READ,
                result: CacheResult::ACCESS,
            })
        ));

        let evt = parse_event("L1-dcache-load-misses").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Cache(Cache {
                which: CacheId::L1D,
                operation: CacheOp::READ,
                result: CacheResult::MISS,
            })
        ));

        let evt = parse_event("LLC-loads").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Cache(Cache {
                which: CacheId::LL,
                operation: CacheOp::READ,
                result: CacheResult::ACCESS,
            })
        ));

        let evt = parse_event("dTLB-load-misses").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Cache(Cache {
                which: CacheId::DTLB,
                operation: CacheOp::READ,
                result: CacheResult::MISS,
            })
        ));
    }

    #[test]
    fn test_parse_raw_events() {
        let evt = parse_event("r1a8").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Raw(Raw { config: 0x1a8, .. })
        ));

        let evt = parse_event("r00c0").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Raw(Raw { config: 0xc0, .. })
        ));

        let evt = parse_event("r0x1a8").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Raw(Raw { config: 0x1a8, .. })
        ));

        let evt = parse_event("R1A8").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Raw(Raw { config: 0x1a8, .. })
        ));
    }

    #[test]
    fn test_parse_events_comma_separated() {
        let events = parse_events("cpu-cycles,instructions,cache-misses").unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[0].event,
            PerfEvent::Hardware(Hardware::CPU_CYCLES)
        ));
        assert!(matches!(
            events[1].event,
            PerfEvent::Hardware(Hardware::INSTRUCTIONS)
        ));
        assert!(matches!(
            events[2].event,
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

    // Tracepoint parsing tests

    #[test]
    fn test_parse_tracepoint_empty_string_returns_error() {
        let result = parse_event("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tracepoint_no_colon_returns_error() {
        let result = parse_event("sched_switch");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tracepoint_empty_subsystem_returns_error() {
        let result = parse_event(":sched_switch");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tracepoint_empty_name_returns_error() {
        let result = parse_event("sched:");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tracepoint_invalid_tracepoint_returns_error() {
        let result = parse_event("invalid:nonexistent_tracepoint");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not found")
                || err_msg.contains("tracefs")
                || err_msg.contains("Unknown event")
        );
    }

    #[test]
    fn test_parse_tracepoint_whitespace_handling() {
        let result = parse_event("  sched:sched_switch  ");
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
    fn test_is_tracepoint_method() {
        let tp = TracepointId::new("sched", "sched_switch", 123);
        let event = PerfEvent::Tracepoint(tp);
        assert!(event.is_tracepoint());
        assert!(!event.is_hardware());
        assert!(!event.is_software());
    }

    #[test]
    fn test_format_tracepoint_event_name() {
        let tp = TracepointId::new("sched", "sched_switch", 123);
        let event = PerfEvent::Tracepoint(tp);
        assert_eq!(format_event_name(&event), "sched:sched_switch");
    }

    #[test]
    fn test_tracepoint_event_clone_eq() {
        let tp1 = TracepointId::new("irq", "irq_handler_entry", 456);
        let event1 = PerfEvent::Tracepoint(tp1);
        let event2 = event1.clone();
        assert_eq!(event1, event2);
    }

    #[test]
    fn test_event_modifier_user_only() {
        let evt = parse_event("cpu-cycles:u").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::CPU_CYCLES)
        ));
        assert!(evt.modifiers.exclude_kernel);
        assert!(!evt.modifiers.exclude_user);
    }

    #[test]
    fn test_event_modifier_kernel_only() {
        let evt = parse_event("instructions:k").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::INSTRUCTIONS)
        ));
        assert!(evt.modifiers.exclude_user);
        assert!(!evt.modifiers.exclude_kernel);
    }

    #[test]
    fn test_event_modifier_hypervisor() {
        let evt = parse_event("cache-misses:h").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::CACHE_MISSES)
        ));
        assert!(evt.modifiers.exclude_user);
        assert!(evt.modifiers.exclude_kernel);
    }

    #[test]
    fn test_event_modifier_user_kernel() {
        let evt = parse_event("cycles:uk").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::CPU_CYCLES)
        ));
        assert!(evt.modifiers.exclude_kernel);
        assert!(evt.modifiers.exclude_user);
    }

    #[test]
    fn test_event_modifier_precise() {
        let evt = parse_event("instructions:p").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::INSTRUCTIONS)
        ));
        assert!(evt.modifiers.precise);
    }

    #[test]
    fn test_event_modifier_combined() {
        let evt = parse_event("cycles:up").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Hardware(Hardware::CPU_CYCLES)
        ));
        assert!(evt.modifiers.exclude_kernel);
        assert!(!evt.modifiers.exclude_user);
        assert!(evt.modifiers.precise);
    }

    #[test]
    fn test_event_modifier_on_cache_event() {
        let evt = parse_event("L1-dcache-loads:u").unwrap();
        assert!(matches!(
            evt.event,
            PerfEvent::Cache(Cache {
                which: CacheId::L1D,
                operation: CacheOp::READ,
                result: CacheResult::ACCESS,
            })
        ));
        assert!(evt.modifiers.exclude_kernel);
    }

    #[test]
    fn test_event_modifier_invalid() {
        let result = parse_event("cycles:xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_event_modifiers_parse() {
        let (mods, unknown) = EventModifiers::parse("uk");
        assert!(mods.exclude_kernel);
        assert!(mods.exclude_user);
        assert!(unknown.is_empty());

        let (mods, unknown) = EventModifiers::parse("p");
        assert!(mods.precise);
        assert!(!mods.exclude_user);
        assert!(!mods.exclude_kernel);

        let (mods, unknown) = EventModifiers::parse("upx");
        assert!(mods.exclude_kernel);
        assert!(mods.precise);
        assert_eq!(unknown, "x");
    }

    #[test]
    fn test_event_modifiers_to_suffix() {
        let mut mods = EventModifiers::new();
        assert!(mods.to_suffix().is_empty());

        mods.exclude_kernel = true;
        assert_eq!(mods.to_suffix(), ":u");

        mods.exclude_kernel = false;
        mods.exclude_user = true;
        assert_eq!(mods.to_suffix(), ":k");

        mods.exclude_kernel = true;
        assert_eq!(mods.to_suffix(), ":h");

        mods.precise = true;
        assert_eq!(mods.to_suffix(), ":hp");
    }

    #[test]
    fn test_tracepoint_with_modifier_not_confused() {
        let evt = parse_event("sched:sched_switch").unwrap();
        assert!(matches!(evt.event, PerfEvent::Tracepoint(_)));
        assert!(evt.modifiers.is_empty());
    }
}
