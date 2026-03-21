//! Script command - dump trace data from recorded perf.data files.
//!
//! This module implements the `perf script` functionality, displaying
//! sample events in a human-readable format with symbol resolution.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use crate::core::perf_data::{
    CommEvent, Event, MmapEvent, PerfDataReader, PerfEventAttr, SampleEvent,
};
use crate::pager::Pager;
use crate::symbols::{MultiResolver, SymbolInfo, SymbolResolver};

const PERF_TYPE_HARDWARE: u32 = 0;
const PERF_TYPE_SOFTWARE: u32 = 1;
const PERF_TYPE_TRACEPOINT: u32 = 2;
const PERF_TYPE_HW_CACHE: u32 = 3;
const PERF_TYPE_RAW: u32 = 4;

fn lookup_tracepoint_name(id: u64) -> Option<String> {
    let tracefs_paths = [
        "/sys/kernel/tracing/events",
        "/sys/kernel/debug/tracing/events",
    ];

    for base_path in &tracefs_paths {
        if !Path::new(base_path).exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(base_path) {
            for subsystem_entry in entries.flatten() {
                let subsystem_path = subsystem_entry.path();
                if !subsystem_path.is_dir() {
                    continue;
                }
                if let Ok(event_entries) = std::fs::read_dir(&subsystem_path) {
                    for event_entry in event_entries.flatten() {
                        let id_path = event_entry.path().join("id");
                        if id_path.exists() {
                            if let Ok(id_str) = std::fs::read_to_string(&id_path) {
                                if let Ok(event_id) = id_str.trim().parse::<u64>() {
                                    if event_id == id {
                                        let subsystem = subsystem_path
                                            .file_name()
                                            .map(|s| s.to_string_lossy().into_owned());
                                        let event =
                                            event_entry.file_name().to_string_lossy().into_owned();
                                        if let Some(s) = subsystem {
                                            return Some(format!("{}:{}", s, event));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn attr_to_event_name(attr: &PerfEventAttr) -> String {
    match attr.type_ {
        PERF_TYPE_HARDWARE => match attr.config {
            0 => "cpu-cycles".to_string(),
            1 => "instructions".to_string(),
            2 => "cache-references".to_string(),
            3 => "cache-misses".to_string(),
            4 => "branch-instructions".to_string(),
            5 => "branch-misses".to_string(),
            6 => "bus-cycles".to_string(),
            7 => "stalled-cycles-frontend".to_string(),
            8 => "stalled-cycles-backend".to_string(),
            9 => "ref-cycles".to_string(),
            _ => format!("hardware:{}", attr.config),
        },
        PERF_TYPE_SOFTWARE => match attr.config {
            0 => "cpu-clock".to_string(),
            1 => "task-clock".to_string(),
            2 => "page-faults".to_string(),
            3 => "context-switches".to_string(),
            4 => "cpu-migrations".to_string(),
            5 => "minor-faults".to_string(),
            6 => "major-faults".to_string(),
            7 => "alignment-faults".to_string(),
            8 => "emulation-faults".to_string(),
            9 => "dummy".to_string(),
            10 => "bpf-output".to_string(),
            _ => format!("software:{}", attr.config),
        },
        PERF_TYPE_TRACEPOINT => lookup_tracepoint_name(attr.config)
            .unwrap_or_else(|| format!("tracepoint:{}", attr.config)),
        PERF_TYPE_HW_CACHE => format!("cache:{}", attr.config),
        PERF_TYPE_RAW => format!("r{:x}", attr.config),
        _ => format!("unknown:{}", attr.type_),
    }
}

/// Format a symbol with optional source location.
fn format_symbol_with_source(info: &SymbolInfo, addr: u64) -> String {
    let offset = addr.saturating_sub(info.start_addr);
    let base = if offset > 0 {
        format!("{}+0x{:x}", info.name, offset)
    } else {
        info.name.clone()
    };

    match (&info.source_file, info.line) {
        (Some(file), Some(line)) => {
            let filename_only = file.rsplit('/').next().unwrap_or(file);
            format!("{} ({}:{})", base, filename_only, line)
        }
        _ => base,
    }
}

/// Resolve an address and format it with symbol info or fallback to hex.
fn resolve_and_format(addr: u64, resolver: &MultiResolver) -> String {
    match resolver.resolve(addr) {
        Ok(Some(sym)) => format_symbol_with_source(&sym, addr),
        _ => format!("0x{:016x}", addr),
    }
}

/// Format timestamp in seconds.nanoseconds format.
fn format_timestamp(nanos: u64) -> String {
    let secs = nanos / 1_000_000_000;
    let subsec = nanos % 1_000_000_000;
    format!("{}.{:09}", secs, subsec)
}

/// Execute the script command.
///
/// Reads a perf.data file and displays sample events in human-readable format.
pub fn execute(
    input: Option<&str>,
    _format: &str,
    show_callchain: bool,
    no_pager: bool,
) -> Result<()> {
    let input_path = input.unwrap_or("perf.data");

    if !Path::new(input_path).exists() {
        anyhow::bail!("Input file not found: {}", input_path);
    }

    let mut reader = PerfDataReader::from_path(input_path)
        .with_context(|| format!("Failed to open {}", input_path))?;

    // Get event name from attrs (use first attr for now - we only support single event)
    let event_name = reader
        .attrs()
        .first()
        .map(attr_to_event_name)
        .unwrap_or_else(|| "unknown".to_string());

    let events = reader
        .read_all_events()
        .with_context(|| "Failed to read events")?;

    let sample_count = events
        .iter()
        .filter(|e| matches!(e, Event::Sample(_)))
        .count();
    if sample_count == 0 {
        println!("No samples in {}", input_path);
        return Ok(());
    }

    let mut comm_map: HashMap<u32, String> = HashMap::new();
    for event in &events {
        if let Event::Comm(comm) = event {
            comm_map.insert(comm.tid, comm.comm.clone());
            comm_map
                .entry(comm.pid)
                .or_insert_with(|| comm.comm.clone());
        }
    }

    let mut resolver = MultiResolver::new();

    let mut kr = crate::symbols::KernelResolver::new();
    if kr.load_symbols(Path::new("")).is_ok() {
        resolver.set_kernel_resolver(kr);
    }

    for event in &events {
        if let Event::Mmap(mmap) = event {
            if !mmap.filename.is_empty() && Path::new(&mmap.filename).exists() {
                let _ = resolver.load_symbols(Path::new(&mmap.filename));
            }
        }
    }

    // Determine if we should use a pager
    let pager = Pager::new();
    let use_pager = !no_pager && pager.should_use_pager();

    // Create output writer - either pager or stdout
    let mut output: Box<dyn Write> = if use_pager {
        pager.spawn().context("Failed to spawn pager")?
    } else {
        Box::new(std::io::stdout())
    };

    for event in &events {
        match event {
            Event::Sample(sample) => {
                display_sample(
                    sample,
                    &comm_map,
                    &resolver,
                    show_callchain,
                    &event_name,
                    &mut output,
                )?;
            }
            Event::Mmap(mmap) => {
                display_mmap(mmap, &mut output)?;
            }
            Event::Comm(comm) => {
                display_comm(comm, &mut output)?;
            }
            _ => {}
        }
    }

    // Flush output before pager is dropped (which waits for child to finish)
    output.flush().context("Failed to flush output")?;

    Ok(())
}

/// Display a sample event in perf script format.
///
/// Format: `comm  PID/TID [CPU] timestamp: event (addr symbol)`
fn display_sample<W: Write>(
    sample: &SampleEvent,
    comm_map: &HashMap<u32, String>,
    resolver: &MultiResolver,
    show_callchain: bool,
    event_name: &str,
    output: &mut W,
) -> Result<()> {
    let comm = comm_map
        .get(&sample.tid)
        .or_else(|| comm_map.get(&sample.pid))
        .cloned()
        .unwrap_or_else(|| format!(":{}", sample.pid));

    let timestamp = format_timestamp(sample.time);
    let symbol = resolve_and_format(sample.ip, resolver);

    writeln!(
        output,
        "{:<16} {:>5}/{:<5} [000] {}: {}: {}",
        comm, sample.pid, sample.tid, timestamp, event_name, symbol
    )?;

    if show_callchain {
        if let Some(ref cc) = sample.callchain {
            for &addr in cc {
                let func = resolve_and_format(addr, resolver);
                writeln!(output, "\t{}", func)?;
            }
        }
    }

    Ok(())
}

/// Display an mmap event.
fn display_mmap<W: Write>(mmap: &MmapEvent, output: &mut W) -> Result<()> {
    writeln!(
        output,
        "MMAP {}/{}: {:016x}-{:016x} {}",
        mmap.pid,
        mmap.tid,
        mmap.addr,
        mmap.addr + mmap.len,
        mmap.filename
    )?;
    Ok(())
}

/// Display a comm event.
fn display_comm<W: Write>(comm: &CommEvent, output: &mut W) -> Result<()> {
    writeln!(output, "COMM {}/{}: {}", comm.pid, comm.tid, comm.comm)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        let ts = format_timestamp(1_123_456_789);
        assert_eq!(ts, "1.123456789");

        let ts = format_timestamp(0);
        assert_eq!(ts, "0.000000000");

        let ts = format_timestamp(1_000_000_000_000);
        assert_eq!(ts, "1000.000000000");
    }

    #[test]
    fn test_format_symbol_with_source_no_offset() {
        let info = SymbolInfo::new("main".to_string(), 0x1000, 0x100);
        let result = format_symbol_with_source(&info, 0x1000);
        assert_eq!(result, "main");
    }

    #[test]
    fn test_format_symbol_with_source_with_offset() {
        let info = SymbolInfo::new("main".to_string(), 0x1000, 0x100);
        let result = format_symbol_with_source(&info, 0x1010);
        assert_eq!(result, "main+0x10");
    }

    #[test]
    fn test_format_symbol_with_source_location() {
        let info = SymbolInfo::new("main".to_string(), 0x1000, 0x100)
            .with_source("src/main.rs".to_string(), 42);
        let result = format_symbol_with_source(&info, 0x1000);
        assert_eq!(result, "main (main.rs:42)");
    }

    #[test]
    fn test_format_symbol_with_source_offset_and_location() {
        let info = SymbolInfo::new("foo".to_string(), 0x2000, 0x50)
            .with_source("/path/to/lib.c".to_string(), 100);
        let result = format_symbol_with_source(&info, 0x2010);
        assert_eq!(result, "foo+0x10 (lib.c:100)");
    }

    #[test]
    fn test_resolve_and_format_no_resolver() {
        let resolver = MultiResolver::new();
        let result = resolve_and_format(0xdeadbeef, &resolver);
        assert_eq!(result, "0x00000000deadbeef");
    }
}
