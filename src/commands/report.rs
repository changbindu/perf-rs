use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::core::perf_data::{Event, PerfDataReader, SampleEvent};
use crate::symbols::{MultiResolver, SymbolResolver};

pub fn execute(
    input: Option<&str>,
    _format: &str,
    sort: Option<&str>,
    top: Option<usize>,
) -> Result<()> {
    let input_path = input.unwrap_or("perf.data");

    if !Path::new(input_path).exists() {
        anyhow::bail!("Input file not found: {}", input_path);
    }

    let mut reader = PerfDataReader::from_path(input_path)
        .with_context(|| format!("Failed to open {}", input_path))?;

    let header = reader.header();
    println!(
        "# Samples: {}, MMAP events: {}, COMM events: {}",
        header.sample_count, header.mmap_count, header.comm_count
    );
    println!();

    let events = reader
        .read_all_events()
        .with_context(|| "Failed to read events")?;

    let samples: Vec<&SampleEvent> = events
        .iter()
        .filter_map(|e| match e {
            Event::Sample(s) => Some(s),
            _ => None,
        })
        .collect();

    if samples.is_empty() {
        println!("No samples found in {}", input_path);
        return Ok(());
    }

    let mut histogram: HashMap<u64, SampleStats> = HashMap::new();
    let total_period: u64 = samples.iter().map(|s| s.period).sum();

    for sample in &samples {
        let entry = histogram.entry(sample.ip).or_default();
        entry.count += 1;
        entry.period += sample.period;
        entry.pid = sample.pid;
        entry.tid = sample.tid;
    }

    let mut sorted: Vec<(&u64, &SampleStats)> = histogram.iter().collect();

    let sort_field = sort.unwrap_or("overhead");
    match sort_field {
        "sample" => sorted.sort_by(|a, b| b.1.count.cmp(&a.1.count)),
        "period" => sorted.sort_by(|a, b| b.1.period.cmp(&a.1.period)),
        _ => sorted.sort_by(|a, b| b.1.period.cmp(&a.1.period)),
    }

    if let Some(n) = top {
        sorted.truncate(n);
    }

    println!(
        "{:>10} {:>10} {:<40} {}",
        "Overhead", "Samples", "Function", "Module"
    );
    println!("{}", "-".repeat(80));

    let mut resolver = MultiResolver::new();
    for (addr, stats) in sorted {
        let overhead = if total_period > 0 {
            (stats.period as f64 / total_period as f64) * 100.0
        } else {
            0.0
        };

        let (func_name, module) = match resolver.resolve(*addr) {
            Ok(Some(info)) => (info.name, String::new()),
            _ => (format!("0x{:016x}", addr), String::new()),
        };

        println!(
            "{:>9.2}% {:>10} {:<40} {}",
            overhead, stats.count, func_name, module
        );
    }

    Ok(())
}

#[derive(Debug, Default)]
struct SampleStats {
    count: u64,
    period: u64,
    pid: u32,
    tid: u32,
}
