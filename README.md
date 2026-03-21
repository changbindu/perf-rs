# perf-rs

> **⚠️ Work in Progress**
>
> This project is currently under active development and is a work-in-progress. It has been developed with heavy assistance from AI tools. Expect incomplete features, potential bugs, and evolving APIs. Use at your own risk.

A Linux performance monitoring tool written in Rust. It provides functionality similar to the `perf` command-line tool, enabling developers to profile and analyze application performance through hardware performance counters.

## Features

- **Event Enumeration**: List available hardware and software performance events
- **Performance Counting**: Count events during command execution or for running processes
- **Sample-Based Profiling**: Record performance samples for detailed analysis
- **Report Generation**: Analyze recorded performance data with symbol resolution
- **Trace Dumping**: Export raw trace data in human-readable or JSON format
- **Architecture Support**: Native support for x86_64, ARM64, and RISC-V 64-bit
- **Symbol Resolution**: Resolve symbols from ELF binaries and kernel symbols
- **Sysfs Integration**: Discover events from sysfs for architecture-specific PMU events

## Requirements

- **Operating System**: Linux
- **Kernel Version**: 5.0 or later
- **Rust Version**: 1.70 or later
- **Privileges**: Root access, `CAP_SYS_ADMIN` capability, or `CAP_PERFMON` capability

### Privilege Details

Performance monitoring requires elevated privileges. The tool checks your privilege level at runtime:

- **Full access**: Root user, `CAP_SYS_ADMIN`, or `CAP_PERFMON` with `perf_event_paranoid <= 1`
- **Limited access**: `perf_event_paranoid = 2` or `CAP_PERFMON` with higher paranoid values
- **No access**: `perf_event_paranoid >= 4` without capabilities

You can check your current `perf_event_paranoid` setting:
```bash
cat /proc/sys/kernel/perf_event_paranoid
```

To adjust it temporarily:
```bash
sudo sysctl -w kernel.perf_event_paranoid=1
```

Or add the `CAP_PERFMON` capability to the binary:
```bash
sudo setcap cap_perfmon+ep $(which perf-rs)
```

## Installation

### From Source

1. Clone the repository:
```bash
git clone https://github.com/changbindu/perf-rs.git
cd perf-rs
```

2. Build the release version:
```bash
cargo build --release
```

3. The binary will be available at `target/release/perf-rs`

### Using Cargo

```bash
cargo install --path .
```

## Usage

### List Available Events

Display available performance events:

```bash
# List all events
sudo perf-rs list

# Filter events by name
sudo perf-rs list --filter cache

# Show detailed event information
sudo perf-rs list --detailed
```

### Count Performance Events

Count events during command execution:

```bash
# Count CPU cycles and instructions
sudo perf-rs stat --event cpu-cycles,instructions -- ./your_program

# Monitor a running process
sudo perf-rs stat --pid 1234 --event cache-misses

# Count multiple events
sudo perf-rs stat --event cpu-cycles,instructions,cache-references,cache-misses -- ./benchmark

# Count tracepoint events (requires root for tracefs access)
sudo perf-rs stat --event sched:sched_switch -- ls

# Count multiple tracepoints
sudo perf-rs stat --event sched:sched_switch,sched:sched_process_fork -- ./your_program
```

### Record Performance Samples

Record samples for profiling:

```bash
# Record at 99 Hz frequency
sudo perf-rs record --frequency 99 -- ./your_program

# Record specific events
sudo perf-rs record --event cpu-cycles --frequency 99 -- ./your_program

# Use sampling period instead of frequency
sudo perf-rs record --event instructions --period 100000 -- ./your_program

# Record from a running process
sudo perf-rs record --pid 1234 --frequency 99

# Specify output file
sudo perf-rs record --output custom.data --frequency 99 -- ./your_program

# Record tracepoint events (requires root for tracefs access)
sudo perf-rs record --event sched:sched_switch -- ./your_program

# Record scheduler tracepoints
sudo perf-rs record --event sched:sched_switch,sched:sched_process_exec -- ./your_program
```

### Analyze Recorded Data

Generate reports from recorded samples:

```bash
# Analyze default perf.data file
sudo perf-rs report

# Specify input file
sudo perf-rs report --input custom.data

# Show top 10 functions
sudo perf-rs report --top 10

# Output as JSON
sudo perf-rs report --format json

# Sort by different fields
sudo perf-rs report --sort sample
```

### Dump Trace Data

Export raw trace data:

```bash
# Dump in text format
sudo perf-rs script

# Show call chains
sudo perf-rs script --callchain

# Output as JSON
sudo perf-rs script --format json

# Specify input file
sudo perf-rs script --input custom.data
```

## Architecture Support

perf-rs supports runtime architecture detection and provides architecture-specific PMU events:

- **x86_64**: Intel and AMD processors with support for architectural and model-specific events
- **ARM64**: ARM Cortex processors with PMUv3 support
- **RISC-V 64**: RISC-V processors with standard performance counters

The tool automatically detects the current architecture and provides relevant events. It also discovers events from sysfs (`/sys/bus/event_source/devices`) for system-specific PMU features.

## Comparison with Standard perf

### Core Comparison

| Feature | perf-rs | Linux perf |
|---------|---------|------------|
| Language | Rust | C |
| Safety | Memory-safe | Manual memory management |
| Dependencies | Minimal (static binary) | Heavy (elfutils, libtraceevent, etc.) |
| Symbol Resolution | Built-in (gimli/addr2line) | Requires external libraries |
| Architecture Support | x86_64, ARM64, RISC-V | All Linux architectures |
| Binary Size | Smaller (~5MB static) | Larger (~15MB + libs) |
| Performance | Comparable | Native |

### Competitive Differentiation

Rather than matching Linux perf feature-for-feature, perf-rs differentiates in areas where Linux perf is fundamentally limited:

| Capability | perf-rs | Linux perf |
|------------|---------|------------|
| **Library API** | ✅ Rust library for embedding | ❌ Command-line only |
| **Remote profiling** | ✅ Planned (agent/server model) | ❌ Local machine only |
| **Multi-host aggregation** | ✅ Planned (cluster profiling) | ❌ Single host |
| **CI/CD integration** | ✅ Planned (regression detection) | ❌ Manual scripting required |
| **Programmatic use** | ✅ Clean Rust API | ❌ Parse text output |
| **Container-aware** | ✅ Planned (K8s native) | ❌ Requires manual setup |
| **Deployment** | Single static binary | Requires system packages |

### Where perf-rs Excels

**1. Embeddable Profiling**
```rust
use perf_rs::{Profiler, Event};

// Profile from within your application
let profiler = Profiler::new()
    .event(Event::CpuCycles)
    .frequency(999)?;

let session = profiler.start()?;
expensive_operation();
let profile = session.stop()?;

let report = profile.analyze()?;
println!("Hottest functions: {:?}", report.top_functions(5));
```

**2. Remote Profiling (Planned)**
```bash
# Run agent on production server
perf-rs agent --server profiling.internal:50051

# Profile remotely from developer machine
perf-rs remote --host prod-server-1 --duration 30s --event cpu-cycles
```

**3. Cluster Profiling (Planned)**
```bash
# Profile multiple hosts simultaneously
perf-rs remote --hosts server1,server2,server3 \
  --event instructions --duration 60s \
  --output cluster-profile.data
```

**4. CI/CD Regression Detection (Planned)**
```bash
# Fail CI if performance regresses > 10%
perf-rs compare baseline.data perf.data --threshold 10%
```

### Where Linux perf Excels

Linux perf remains superior for:
- BPF/eBPF program support
- All Linux architecture support
- Kernel developer workflows
- Mature ecosystem and documentation

### Strategic Position

perf-rs targets a different use case than Linux perf:

| Use Case | Recommended Tool |
|----------|------------------|
| Kernel debugging | Linux perf |
| BPF program development | Linux perf |
| One-off local profiling | Linux perf or perf-rs |
| Embedded profiling in applications | perf-rs |
| Production/remote profiling | perf-rs |
| Cluster-wide analysis | perf-rs |
| CI/CD performance gates | perf-rs |

### Advantages of perf-rs

- Memory-safe implementation in Rust
- Minimal dependencies, static binary deployment
- Self-contained symbol resolution
- Clean, modern codebase
- Embeddable as library
- Designed for remote and distributed profiling

### Limitations vs Linux perf

- Fewer supported architectures (3 vs 20+)
- No BPF/eBPF program support (not planned)
- Fewer commands (5 vs 22)

## Current Status

### Commands Coverage (5/7 = 71% of core commands)

| Command | Status | Description |
|---------|--------|-------------|
| `list` | ✅ Complete | Hardware, software, cache, raw, tracepoint events with filtering & pagination |
| `stat` | ✅ Complete | Per-process, system-wide, per-CPU counting modes |
| `record` | ✅ Complete | Frequency/period sampling, call graphs (-g), system-wide |
| `report` | ✅ Complete | Symbol resolution, overhead calculation, JSON output |
| `script` | ✅ Complete | Text/JSON output with callchains |
| `diff` | ❌ Planned | Compare perf.data files |
| `evlist` | ❌ Planned | List events in file |

### Event Types

| Category | Status | Details |
|----------|--------|---------|
| Hardware events | ✅ Complete | cpu-cycles, instructions, cache-refs/misses, branches, bus-cycles, ref-cycles, stalled-cycles |
| Software events | ✅ Complete | cpu-clock, task-clock, page-faults, context-switches, cpu-migrations, minor/major-faults, bpf-output |
| Cache events | ✅ Complete | L1-dcache, L1-icache, LLC, dTLB, iTLB, branch, node (all variants) |
| Raw events | ✅ Complete | rNNNN format for architecture-specific PMU events |
| Tracepoint events | ✅ Complete | syscalls, sched, irq, timer, net, etc. (format: subsystem:event) |
| kprobes | ❌ Planned | Kernel dynamic tracepoints |
| uprobes | ❌ Planned | Userspace dynamic tracepoints |

### Sampling Features

| Feature | Status | Details |
|---------|--------|---------|
| Frequency-based sampling | ✅ Complete | `-F/--frequency` Hz |
| Period-based sampling | ✅ Complete | `-c/--period` events |
| Per-process (`-p/--pid`) | ✅ Complete | Attach to running process |
| Per-CPU (`-C/--cpu`) | ✅ Complete | Specific CPUs |
| System-wide (`-a/--all-cpus`) | ✅ Complete | All CPUs |
| Command execution | ✅ Complete | `-- <cmd>` profiling |
| Call graphs (`-g`) | ✅ Complete | Frame pointer unwinding |
| Sample: IP, TID, TIME, PERIOD | ✅ Complete | Core sample data |
| Sample: CPU, CALLCHAIN | ✅ Complete | Extended sample data |
| LBR (Last Branch Record) | ❌ Planned | Branch trace capture |
| PEBS | ❌ Planned | Precise Event-Based Sampling |
| Intel PT | ❌ Planned | Full execution trace |
| Event modifiers (:u, :k, :p) | ❌ Planned | User/kernel/precise modifiers |
| Event groups ({e1,e2}) | ❌ Planned | Synchronized event groups |

### Core Features

| Feature | Status | Notes |
|---------|--------|-------|
| perf.data read/write | ✅ Complete | PERFILE2 format, Linux perf compatible |
| Symbol resolution (ELF) | ✅ Complete | Symbol table + DWARF debug info |
| Symbol resolution (kernel) | ✅ Complete | /proc/kallsyms parsing |
| Ring buffer sampling | ✅ Complete | Per-PID and per-CPU modes |
| Privilege checking | ✅ Complete | Root, CAP_SYS_ADMIN, CAP_PERFMON detection |

### Architecture Support

| Architecture | Status | PMU Events |
|--------------|--------|------------|
| x86_64 | ✅ Complete | Intel + AMD events + sysfs discovery |
| ARM64 | ✅ Complete | Cortex-A + Neoverse events + sysfs discovery |
| RISC-V 64 | ✅ Complete | Standard + SiFive events + sysfs discovery |
| x86 (32-bit) | ❌ Not Planned | - |
| ARM (32-bit) | ❌ Not Planned | - |
| PowerPC | ❌ Not Planned | - |
| s390 | ❌ Not Planned | - |

### Output & UX

| Feature | Status | Notes |
|---------|--------|-------|
| Text output | ✅ Complete | Default for all commands |
| JSON output | ✅ Complete | `--format json` for report/script |
| Pagination | ✅ Complete | list, report, script with pager |
| CSV output | ❌ Planned | Export to CSV format |
| Flame graphs | ❌ Planned | Visualization support |
| Chrome tracing | ❌ Planned | Chrome DevTools format |
| TUI interface | ❌ Planned | Interactive report viewer |

### Out of Scope

| Feature | Reason |
|---------|--------|
| BPF/eBPF program support | Requires kernel integration beyond profiling |
| Kernel module requirements | User-space tool design |
| DWARF call stack unwinding | Frame pointer sufficient for most cases |

**Status Legend**: ✅ Complete | ❌ Planned | ⏸️ Not Planned

## Development Plan

The development roadmap prioritizes **differentiation over feature parity**. perf-rs focuses on capabilities that Linux perf cannot provide: embeddable library API, remote profiling, and CI/CD integration.

### Differentiation Features

| Feature | Status | Description |
|---------|--------|-------------|
| Library API | ❌ Planned | Rust library for embedding profiling in applications |
| Remote agent | ❌ Planned | Profile production servers over network |
| Remote server | ❌ Planned | Central collection and analysis server |
| Multi-host profiling | ❌ Planned | Profile clusters simultaneously |
| Regression detection | ❌ Planned | CI/CD performance gates |

### Phase 1: Library API (v0.2) - Foundation

Expose clean Rust API for programmatic profiling:

- **Profiler API**
  - `Profiler::new()` builder pattern
  - `profiler.start()` → `Session`
  - `session.stop()` → `ProfileData`
  - `ProfileData.analyze()` → `Report`

- **Async Support**
  - Tokio integration for async profiling
  - Non-blocking sample collection

- **Examples**
  - Embedded profiler in application
  - Custom analysis pipeline
  - Integration with tracing/logging

### Phase 2: Remote Profiling (v0.3) - Core Differentiation

Enable profiling over network:

- **Agent Mode**
  - `perf-rs agent --server <addr>` - lightweight daemon
  - Accept remote commands (start/stop/status)
  - Stream samples over gRPC

- **Server Mode**
  - `perf-rs server` - central collection point
  - Aggregate samples from multiple agents
  - REST API for triggering profiles

- **Remote Commands**
  - `perf-rs remote --host <ip> --duration 30s`
  - TLS encryption for transport
  - Authentication (token-based)

### Phase 3: Multi-Host & CI/CD (v0.4) - Automation

Scale to clusters and pipelines:

- **Cluster Profiling**
  - Profile multiple hosts in parallel
  - `perf-rs remote --hosts host1,host2,host3`
  - Aggregate reports across cluster

- **Regression Detection**
  - `perf-rs compare baseline.data current.data`
  - Configurable thresholds (--threshold 10%)
  - Exit codes for CI pass/fail

- **CI/CD Integration**
  - GitHub Actions integration
  - GitLab CI templates
  - Historical trend tracking

### Phase 4: Enhanced Features (v0.5+)

- **Container Support**
  - Profile containers from host
  - K8s operator for cluster profiling
  - Namespace-aware sampling

- **Output Formats**
  - Flame graph SVG generation
  - Chrome tracing format
  - pprof compatibility

### Feature Parity Goals (Secondary)

These Linux perf features are planned but secondary to differentiation:

- **Event Modifiers** - `:u`, `:k`, `:p` modifiers
- **TUI Interface** - Interactive report viewer

### Not Planned

Features that don't align with perf-rs's differentiation strategy:

- BPF/eBPF program support (kernel subsystem, use Linux perf)
- Kernel module requirements (violates user-space design)
- Full Intel PT decoding (extensive decoder complexity)
- All Linux architecture support (focus on x86_64, ARM64, RISC-V)

## Project Structure

```
perf-rs/
├── src/
│   ├── main.rs           # Entry point and command dispatch
│   ├── cli.rs            # CLI argument definitions
│   ├── error.rs          # Custom error types
│   ├── arch/             # Architecture-specific code
│   │   ├── mod.rs
│   │   ├── x86_64.rs     # x86_64 PMU events
│   │   ├── arm64.rs      # ARM64 PMU events
│   │   └── riscv64.rs    # RISC-V PMU events
│   ├── commands/         # Subcommand implementations
│   │   ├── mod.rs
│   │   ├── list.rs       # perf list
│   │   ├── stat.rs       # perf stat
│   │   ├── record.rs     # perf record
│   │   ├── report.rs     # perf report
│   │   └── script.rs     # perf script
│   ├── core/             # Core functionality
│   │   ├── mod.rs
│   │   ├── perf_event.rs # Performance counter API
│   │   ├── perf_data.rs  # perf.data file handling
│   │   ├── ringbuf.rs    # Ring buffer for sampling
│   │   └── privilege.rs  # Privilege checking
│   └── symbols/          # Symbol resolution
│       ├── mod.rs
│       ├── elf.rs        # ELF symbol resolver
│       └── kernel.rs     # Kernel symbol resolver
└── Cargo.toml
```

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Linting

```bash
cargo clippy
cargo fmt
```

### Documentation

```bash
cargo doc --open
```

## Known Issues

1. **Permission Errors**: Always check privileges before running commands. Use `--verbose` for detailed error messages.

2. **Kernel Version**: Some features require kernel 5.0+. Older kernels may have limited functionality.

3. **Symbol Resolution**: Debug symbols must be present in binaries for accurate symbol resolution. Strip binaries will show raw addresses.

4. **Event Availability**: Not all events are available on all systems. Use `perf-rs list` to see available events on your system.

5. **High-Frequency Sampling**: Very high sampling frequencies (>10000 Hz) may cause overhead. Start with 99-999 Hz.

## Contributing

Contributions are welcome! Please read the code style guidelines in `AGENTS.md` and ensure:

- All code passes `cargo clippy` without warnings
- Code is formatted with `cargo fmt`
- New features include tests
- Public APIs are documented with doc comments

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Inspired by the Linux `perf` tool
- Uses the `perf-event2` crate for Linux perf event API access
- Symbol resolution powered by `gimli` and `addr2line`
- CLI interface built with `clap`
