use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use crate::core::perf_data::{Event, PerfDataReader, SampleEvent};
use crate::pager::Pager;
use crate::symbols::{MultiResolver, SymbolInfo, SymbolResolver};

fn format_symbol_with_source(info: &SymbolInfo) -> String {
    match (&info.source_file, info.line) {
        (Some(file), Some(line)) => {
            let filename_only = file.rsplit('/').next().unwrap_or(file);
            format!("{} ({}:{})", info.name, filename_only, line)
        }
        _ => info.name.clone(),
    }
}

fn resolve_and_format(addr: u64, resolver: &MultiResolver) -> String {
    match resolver.resolve(addr) {
        Ok(Some(sym)) => format_symbol_with_source(&sym),
        _ => format!("0x{:016x}", addr),
    }
}

pub fn execute(
    input: Option<&str>,
    _format: &str,
    sort: Option<&str>,
    top: Option<usize>,
    no_pager: bool,
) -> Result<()> {
    let input_path = input.unwrap_or("perf.data");

    if !Path::new(input_path).exists() {
        anyhow::bail!("Input file not found: {}", input_path);
    }

    let mut reader = PerfDataReader::from_path(input_path)
        .with_context(|| format!("Failed to open {}", input_path))?;

    let events = reader
        .read_all_events()
        .with_context(|| "Failed to read events")?;

    let sample_count = events
        .iter()
        .filter(|e| matches!(e, Event::Sample(_)))
        .count();
    let mmap_count = events
        .iter()
        .filter(|e| matches!(e, Event::Mmap(_)))
        .count();
    let comm_count = events
        .iter()
        .filter(|e| matches!(e, Event::Comm(_)))
        .count();

    println!(
        "# Samples: {}, MMAP events: {}, COMM events: {}",
        sample_count, mmap_count, comm_count
    );
    println!();

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

    let mut call_graph = CallGraph::new();

    // First, populate mmap and comm mappings
    for event in &events {
        match event {
            Event::Mmap(mmap) => {
                if !mmap.filename.is_empty() {
                    call_graph.add_mmap(mmap.addr, mmap.len, mmap.filename.clone());
                }
            }
            Event::Comm(comm) => {
                call_graph.add_comm(comm.pid, comm.comm.clone());
            }
            _ => {}
        }
    }

    let total_period: u64 = samples.iter().map(|s| s.period).sum();

    for sample in &samples {
        call_graph.add_sample(sample, &resolver);
    }

    call_graph.sort_and_display(sort, top, total_period, &resolver, no_pager)?;

    Ok(())
}

#[derive(Debug, Default, Clone)]
struct FunctionStats {
    self_count: u64,
    self_period: u64,
    total_count: u64,
    total_period: u64,
    callchain_hits: u64,
    pid: u32,
    tid: u32,
    shared_object: Option<String>,
    comm: Option<String>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct CallEdge {
    caller: u64,
    callee: u64,
}

#[derive(Debug)]
struct MmapInfo {
    addr: u64,
    len: u64,
    filename: String,
}

#[derive(Debug)]
struct CallGraph {
    functions: HashMap<u64, FunctionStats>,
    edges: HashMap<CallEdge, u64>,
    total_samples: u64,
    mmap_events: Vec<MmapInfo>,
    comm_map: HashMap<u32, String>,
}

impl CallGraph {
    fn new() -> Self {
        Self {
            functions: HashMap::new(),
            edges: HashMap::new(),
            total_samples: 0,
            mmap_events: Vec::new(),
            comm_map: HashMap::new(),
        }
    }

    fn add_mmap(&mut self, addr: u64, len: u64, filename: String) {
        self.mmap_events.push(MmapInfo {
            addr,
            len,
            filename,
        });
    }

    fn add_comm(&mut self, pid: u32, comm: String) {
        self.comm_map.insert(pid, comm);
    }

    fn find_shared_object(&self, addr: u64) -> Option<String> {
        for mmap in &self.mmap_events {
            if addr >= mmap.addr && addr < mmap.addr + mmap.len {
                return Some(mmap.filename.clone());
            }
        }
        None
    }

    fn get_comm(&self, pid: u32) -> Option<String> {
        self.comm_map.get(&pid).cloned()
    }

    fn add_sample(&mut self, sample: &SampleEvent, _resolver: &MultiResolver) {
        self.total_samples += 1;

        let shared_obj = self.find_shared_object(sample.ip);
        let comm = self.get_comm(sample.pid);

        let ip_stats = self.functions.entry(sample.ip).or_default();
        ip_stats.self_count += 1;
        ip_stats.self_period += sample.period;
        ip_stats.total_count += 1;
        ip_stats.total_period += sample.period;
        ip_stats.pid = sample.pid;
        ip_stats.tid = sample.tid;

        if ip_stats.shared_object.is_none() {
            ip_stats.shared_object = shared_obj;
        }
        if ip_stats.comm.is_none() {
            ip_stats.comm = comm;
        }

        if sample.callchain.as_ref().map_or(false, |cc| !cc.is_empty()) {
            if let Some(ref cc) = sample.callchain {
                for (i, &addr) in cc.iter().enumerate() {
                    let shared_obj = self.find_shared_object(addr);
                    let comm = self.get_comm(sample.pid);

                    let stats = self.functions.entry(addr).or_default();

                    if i > 0 {
                        stats.callchain_hits += 1;
                        stats.total_count += 1;
                        stats.total_period += sample.period;
                    }

                    if stats.shared_object.is_none() {
                        stats.shared_object = shared_obj;
                    }
                    if stats.comm.is_none() {
                        stats.comm = comm;
                    }

                    if i + 1 < cc.len() {
                        let edge = CallEdge {
                            caller: cc[i + 1],
                            callee: addr,
                        };
                        *self.edges.entry(edge).or_default() += 1;
                    }
                }
            }
        }
    }

    fn sort_and_display(
        &self,
        sort: Option<&str>,
        top: Option<usize>,
        total_period: u64,
        resolver: &MultiResolver,
        no_pager: bool,
    ) -> Result<()> {
        let mut output: Box<dyn Write> = if !no_pager && Pager::new().should_use_pager() {
            Pager::new().spawn()?
        } else {
            Box::new(std::io::stdout())
        };

        let mut sorted: Vec<(&u64, &FunctionStats)> = self.functions.iter().collect();

        let sort_field = sort.unwrap_or("overhead");
        match sort_field {
            "sample" => sorted.sort_by(|a, b| b.1.self_count.cmp(&a.1.self_count)),
            "period" => sorted.sort_by(|a, b| b.1.self_period.cmp(&a.1.self_period)),
            "overhead" => sorted.sort_by(|a, b| {
                let a_overhead = if total_period > 0 {
                    (b.1.self_period as f64 / total_period as f64) * 100.0
                } else {
                    0.0
                };
                let b_overhead = if total_period > 0 {
                    (a.1.self_period as f64 / total_period as f64) * 100.0
                } else {
                    0.0
                };
                a_overhead
                    .partial_cmp(&b_overhead)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            _ => sorted.sort_by(|a, b| b.1.self_period.cmp(&a.1.self_period)),
        }

        if let Some(n) = top {
            sorted.truncate(n);
        }

        writeln!(
            output,
            "{:>8} {:>8} {:>8} {:>8} {:<30} {:<20} {:<15}",
            "Overhead", "Self", "Total", "Samples", "Function", "Shared Object", "Comm"
        )?;
        writeln!(output, "{}", "-".repeat(110))?;

        for (addr, stats) in &sorted {
            let overhead = if total_period > 0 {
                (stats.self_period as f64 / total_period as f64) * 100.0
            } else {
                0.0
            };

            let total_overhead = if total_period > 0 {
                (stats.total_period as f64 / total_period as f64) * 100.0
            } else {
                0.0
            };

            let symbol_name = resolve_and_format(**addr, resolver);
            let shared_obj = stats
                .shared_object
                .as_ref()
                .map(|s| {
                    let filename = s.rsplit('/').next().unwrap_or(s);
                    if filename.len() > 18 {
                        format!("...{}", &filename[filename.len() - 15..])
                    } else {
                        filename.to_string()
                    }
                })
                .unwrap_or_else(|| "[unknown]".to_string());
            let comm = stats.comm.as_deref().unwrap_or("[unknown]");

            writeln!(
                output,
                "{:>7.2}% {:>7.2}% {:>7.2}% {:>8} {:<30} {:<20} {:<15}",
                overhead,
                total_overhead,
                if stats.self_period > 0 {
                    (stats.self_period as f64 / total_period as f64) * 100.0
                } else {
                    0.0
                },
                stats.self_count,
                symbol_name,
                shared_obj,
                comm
            )?;
        }

        if !self.edges.is_empty() && !sorted.is_empty() {
            writeln!(output)?;
            self.display_call_graph(&sorted, resolver, &mut output)?;
        }

        Ok(())
    }

    fn display_call_graph(
        &self,
        top_functions: &[(&u64, &FunctionStats)],
        resolver: &MultiResolver,
        output: &mut Box<dyn Write>,
    ) -> Result<()> {
        writeln!(output, "# Call Graph")?;
        writeln!(output, "{}", "=".repeat(80))?;
        writeln!(output)?;

        let display_count = std::cmp::min(top_functions.len(), 5);

        for (idx, (addr, _stats)) in top_functions.iter().take(display_count).enumerate() {
            if idx > 0 {
                writeln!(output)?;
            }

            let callers: Vec<(u64, u64)> = self
                .edges
                .iter()
                .filter(|(edge, _)| edge.callee == **addr)
                .map(|(edge, &count)| (edge.caller, count))
                .collect();

            let callees: Vec<(u64, u64)> = self
                .edges
                .iter()
                .filter(|(edge, _)| edge.caller == **addr)
                .map(|(edge, &count)| (edge.callee, count))
                .collect();

            let func_name = resolve_and_format(**addr, resolver);

            writeln!(
                output,
                "{} [{}]",
                func_name,
                self.functions.get(addr).map(|s| s.self_count).unwrap_or(0)
            )?;

            if !callers.is_empty() {
                let mut sorted_callers: Vec<_> = callers.into_iter().collect();
                sorted_callers.sort_by(|a, b| b.1.cmp(&a.1));

                writeln!(output, "  Callers:")?;
                for (caller_addr, count) in sorted_callers.iter().take(5) {
                    let caller_name = resolve_and_format(*caller_addr, resolver);
                    let stats = self.functions.get(caller_addr);
                    let percent = if let Some(s) = stats {
                        if s.total_count > 0 {
                            (*count as f64 / s.total_count as f64) * 100.0
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    writeln!(
                        output,
                        "    ├── {} ({} calls, {:.1}%)",
                        caller_name, count, percent
                    )?;
                }
            }

            if !callees.is_empty() {
                let mut sorted_callees: Vec<_> = callees.into_iter().collect();
                sorted_callees.sort_by(|a, b| b.1.cmp(&a.1));

                writeln!(output, "  Callees:")?;
                for (callee_addr, count) in sorted_callees.iter().take(5) {
                    let callee_name = resolve_and_format(*callee_addr, resolver);
                    let stats = self.functions.get(callee_addr);
                    let percent = if let Some(s) = stats {
                        if s.total_count > 0 {
                            (*count as f64 / s.total_count as f64) * 100.0
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    writeln!(
                        output,
                        "    ├── {} ({} calls, {:.1}%)",
                        callee_name, count, percent
                    )?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_stats_default() {
        let stats = FunctionStats::default();
        assert_eq!(stats.self_count, 0);
        assert_eq!(stats.self_period, 0);
        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.total_period, 0);
        assert_eq!(stats.callchain_hits, 0);
    }

    #[test]
    fn test_call_graph_new() {
        let graph = CallGraph::new();
        assert_eq!(graph.total_samples, 0);
        assert!(graph.functions.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_call_graph_add_sample_no_callchain() {
        let mut graph = CallGraph::new();
        let resolver = MultiResolver::new();

        let sample_type = crate::core::perf_data::PERF_SAMPLE_IP
            | crate::core::perf_data::PERF_SAMPLE_TID
            | crate::core::perf_data::PERF_SAMPLE_TIME
            | crate::core::perf_data::PERF_SAMPLE_PERIOD;

        let sample = SampleEvent::new(sample_type, 100, 0x1000, 1234, 5678, 1000, None, None);
        graph.add_sample(&sample, &resolver);

        assert_eq!(graph.total_samples, 1);
        assert!(graph.functions.contains_key(&0x1000));

        let stats = graph.functions.get(&0x1000).unwrap();
        assert_eq!(stats.self_count, 1);
        assert_eq!(stats.self_period, 1000);
        assert_eq!(stats.total_count, 1);
        assert_eq!(stats.total_period, 1000);
        assert_eq!(stats.callchain_hits, 0);
    }

    #[test]
    fn test_call_graph_add_sample_with_callchain() {
        let mut graph = CallGraph::new();
        let resolver = MultiResolver::new();

        let callchain = vec![0x1000, 0x2000, 0x3000];

        let sample_type = crate::core::perf_data::PERF_SAMPLE_IP
            | crate::core::perf_data::PERF_SAMPLE_TID
            | crate::core::perf_data::PERF_SAMPLE_TIME
            | crate::core::perf_data::PERF_SAMPLE_PERIOD
            | crate::core::perf_data::PERF_SAMPLE_CALLCHAIN;

        let sample = SampleEvent::new(
            sample_type,
            100,
            0x1000,
            1234,
            5678,
            1000,
            Some(callchain),
            None,
        );
        graph.add_sample(&sample, &resolver);

        assert_eq!(graph.total_samples, 1);

        let ip_stats = graph.functions.get(&0x1000).unwrap();
        assert_eq!(ip_stats.self_count, 1);
        assert_eq!(ip_stats.self_period, 1000);
        assert_eq!(ip_stats.total_count, 1);
        assert_eq!(ip_stats.callchain_hits, 0);

        let caller1_stats = graph.functions.get(&0x2000).unwrap();
        assert_eq!(caller1_stats.self_count, 0);
        assert_eq!(caller1_stats.self_period, 0);
        assert_eq!(caller1_stats.total_count, 1);
        assert_eq!(caller1_stats.total_period, 1000);
        assert_eq!(caller1_stats.callchain_hits, 1);

        let caller2_stats = graph.functions.get(&0x3000).unwrap();
        assert_eq!(caller2_stats.self_count, 0);
        assert_eq!(caller2_stats.self_period, 0);
        assert_eq!(caller2_stats.total_count, 1);
        assert_eq!(caller2_stats.total_period, 1000);
        assert_eq!(caller2_stats.callchain_hits, 1);

        assert_eq!(graph.edges.len(), 2);
        assert_eq!(
            *graph
                .edges
                .get(&CallEdge {
                    caller: 0x2000,
                    callee: 0x1000
                })
                .unwrap(),
            1
        );
        assert_eq!(
            *graph
                .edges
                .get(&CallEdge {
                    caller: 0x3000,
                    callee: 0x2000
                })
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_call_edge_hash() {
        let edge1 = CallEdge {
            caller: 0x1000,
            callee: 0x2000,
        };
        let edge2 = CallEdge {
            caller: 0x1000,
            callee: 0x2000,
        };
        let edge3 = CallEdge {
            caller: 0x1000,
            callee: 0x3000,
        };

        assert_eq!(edge1, edge2);
        assert_ne!(edge1, edge3);
    }

    #[test]
    fn test_multiple_samples_aggregation() {
        let mut graph = CallGraph::new();
        let resolver = MultiResolver::new();

        let sample_type = crate::core::perf_data::PERF_SAMPLE_IP
            | crate::core::perf_data::PERF_SAMPLE_TID
            | crate::core::perf_data::PERF_SAMPLE_TIME
            | crate::core::perf_data::PERF_SAMPLE_PERIOD;

        let sample1 = SampleEvent::new(sample_type, 100, 0x1000, 1234, 5678, 500, None, None);
        graph.add_sample(&sample1, &resolver);

        let callchain = vec![0x1000, 0x2000];

        let sample_type_with_callchain =
            sample_type | crate::core::perf_data::PERF_SAMPLE_CALLCHAIN;

        let sample2 = SampleEvent::new(
            sample_type_with_callchain,
            200,
            0x1000,
            1234,
            5678,
            500,
            Some(callchain),
            None,
        );
        graph.add_sample(&sample2, &resolver);

        assert_eq!(graph.total_samples, 2);

        let stats_1000 = graph.functions.get(&0x1000).unwrap();
        assert_eq!(stats_1000.self_count, 2);
        assert_eq!(stats_1000.self_period, 1000);
        assert_eq!(stats_1000.total_count, 2);
        assert_eq!(stats_1000.total_period, 1000);
        assert_eq!(stats_1000.callchain_hits, 0);

        let stats_2000 = graph.functions.get(&0x2000).unwrap();
        assert_eq!(stats_2000.self_count, 0);
        assert_eq!(stats_2000.self_period, 0);
        assert_eq!(stats_2000.total_count, 1);
        assert_eq!(stats_2000.total_period, 500);
        assert_eq!(stats_2000.callchain_hits, 1);
    }
}
