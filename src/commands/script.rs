use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::core::perf_data::{CommEvent, Event, MmapEvent, PerfDataReader, SampleEvent};
use crate::symbols::{MultiResolver, SymbolResolver};

pub fn execute(input: Option<&str>, _format: &str, show_callchain: bool) -> Result<()> {
    let input_path = input.unwrap_or("perf.data");

    if !Path::new(input_path).exists() {
        anyhow::bail!("Input file not found: {}", input_path);
    }

    let mut reader = PerfDataReader::from_path(input_path)
        .with_context(|| format!("Failed to open {}", input_path))?;

    let events = reader
        .read_all_events()
        .with_context(|| "Failed to read events")?;

    let mut comm_map: HashMap<u32, String> = HashMap::new();
    let resolver = MultiResolver::new();

    for event in &events {
        match event {
            Event::Comm(comm) => {
                comm_map.insert(comm.pid, comm.comm.clone());
            }
            Event::Mmap(_mmap) => {}
            _ => {}
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
        }
    }

    Ok(())
}

fn display_sample(
    sample: &SampleEvent,
    comm_map: &HashMap<u32, String>,
    resolver: &MultiResolver,
    show_callchain: bool,
) {
    let comm = comm_map
        .get(&sample.pid)
        .cloned()
        .unwrap_or_else(|| format!("pid:{}", sample.pid));

    let timestamp = sample.header.time;

    let (func_name, _offset) = match resolver.resolve(sample.ip) {
        Ok(Some(info)) => {
            let offset = sample.ip.saturating_sub(info.start_addr);
            (info.name, offset)
        }
        _ => (format!("0x{:016x}", sample.ip), 0),
    };

    println!(
        "{} {}/{} [{:03}] {:>12}: {:>16} {}",
        comm, sample.pid, sample.tid, 0, timestamp, sample.ip, func_name
    );

    if show_callchain && !sample.callchain.is_empty() {
        for addr in &sample.callchain {
            let func = match resolver.resolve(*addr) {
                Ok(Some(info)) => info.name,
                _ => format!("0x{:016x}", addr),
            };
            println!("\t{}", func);
        }
    }
}

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

fn display_comm(comm: &CommEvent) {
    println!("COMM {}/{}: {}", comm.pid, comm.tid, comm.comm);
}
