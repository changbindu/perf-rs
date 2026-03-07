//! Script command - dump trace data from recorded perf.data files.
//!
//! This module implements the `perf script` functionality, displaying
//! sample events in a human-readable format with symbol resolution.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::core::perf_data::{CommEvent, Event, MmapEvent, PerfDataReader, SampleEvent};
use crate::symbols::{MultiResolver, SymbolInfo, SymbolResolver};

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
pub fn execute(input: Option<&str>, _format: &str, show_callchain: bool) -> Result<()> {
    let input_path = input.unwrap_or("perf.data");

    if !Path::new(input_path).exists() {
        anyhow::bail!("Input file not found: {}", input_path);
    }

    let mut reader = PerfDataReader::from_path(input_path)
        .with_context(|| format!("Failed to open {}", input_path))?;

    let header = reader.header();
    if header.sample_count == 0 {
        println!("No samples in {}", input_path);
        return Ok(());
    }

    let events = reader
        .read_all_events()
        .with_context(|| "Failed to read events")?;

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
    for event in &events {
        match event {
            Event::Sample(sample) => {
                display_sample(sample, &comm_map, &resolver, show_callchain);
            }
            Event::Mmap(mmap) => {
                display_mmap(mmap);
            }
            Event::Comm(comm) => {
                display_comm(comm);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Display a sample event in perf script format.
///
/// Format: `comm  PID/TID [CPU] timestamp: event (addr symbol)`
fn display_sample(
    sample: &SampleEvent,
    comm_map: &HashMap<u32, String>,
    resolver: &MultiResolver,
    show_callchain: bool,
) {
    let comm = comm_map
        .get(&sample.tid)
        .or_else(|| comm_map.get(&sample.pid))
        .cloned()
        .unwrap_or_else(|| format!(":{}", sample.pid));

    let timestamp = format_timestamp(sample.time);
    let symbol = resolve_and_format(sample.ip, resolver);
    let event_name = "cycles";

    println!(
        "{:<16} {:>5}/{:<5} [000] {}: {}: {}",
        comm, sample.pid, sample.tid, timestamp, event_name, symbol
    );

    if show_callchain {
        if let Some(ref cc) = sample.callchain {
            for &addr in cc {
                let func = resolve_and_format(addr, resolver);
                println!("\t{}", func);
            }
        }
    }
}

/// Display an mmap event.
fn display_mmap(mmap: &MmapEvent) {
    println!(
        "MMAP {}/{}: {:016x}-{:016x} {}",
        mmap.pid,
        mmap.tid,
        mmap.addr,
        mmap.addr + mmap.len,
        mmap.filename
    );
}

/// Display a comm event.
fn display_comm(comm: &CommEvent) {
    println!("COMM {}/{}: {}", comm.pid, comm.tid, comm.comm);
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
