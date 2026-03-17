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

| Feature | perf-rs | Linux perf |
|---------|---------|------------|
| Language | Rust | C |
| Safety | Memory-safe | Manual memory management |
| Dependencies | Minimal | Heavy (elfutils, libtraceevent, etc.) |
| Symbol Resolution | Built-in (gimli/addr2line) | Requires external libraries |
| Architecture Support | x86_64, ARM64, RISC-V | All Linux architectures |
| Event Discovery | Sysfs + builtin | Sysfs + tracepoint |
| Binary Size | Smaller | Larger |
| Performance | Comparable | Native |

### Advantages of perf-rs

- Memory-safe implementation in Rust
- Minimal dependencies
- Self-contained symbol resolution
- Clean, modern codebase
- Easier to extend and maintain

### Limitations

- Fewer supported architectures than standard perf
- Limited tracepoint support
- No live mode or TUI reporter (yet)
- Basic event filtering compared to perf
- No support for BPF or eBPF programs

## Current Status

### Commands Coverage (5/22 = 23% of Linux perf commands)

| Command | Status | Description |
|---------|--------|-------------|
| `list` | ✅ Complete | Hardware, software, cache, raw events with filtering & pagination |
| `stat` | ✅ Complete | Per-process, system-wide, per-CPU counting modes |
| `record` | ✅ Complete | Frequency/period sampling, call graphs (-g), system-wide |
| `report` | ✅ Complete | Symbol resolution, overhead calculation, JSON output |
| `script` | ✅ Complete | Text/JSON output with callchains |
| `top` | ❌ Planned | Live real-time profiling |
| `annotate` | ❌ Planned | Source annotation with samples |
| `mem` | ❌ Planned | Memory access profiling |
| `sched` | ❌ Planned | Scheduler profiling |
| `lock` | ❌ Planned | Lock contention analysis |
| `kmem` | ❌ Planned | Kernel memory profiling |
| `kvm` | ❌ Planned | KVM guest profiling |
| `bench` | ❌ Planned | Built-in benchmarks |
| `trace` | ❌ Planned | System call tracing |
| `diff` | ❌ Planned | Compare perf.data files |
| `inject` | ❌ Planned | Modify events in trace |
| `probe` | ❌ Planned | Dynamic tracepoints |
| `ftrace` | ❌ Planned | ftrace wrapper |
| `timechart` | ❌ Planned | Visualization |
| `data` | ❌ Planned | Data file conversion |
| `evlist` | ❌ Planned | List events in file |
| `buildid-cache` | ❌ Planned | Build-id cache management |

### Event Types

| Category | Status | Details |
|----------|--------|---------|
| Hardware events | ✅ Complete | cpu-cycles, instructions, cache-refs/misses, branches, bus-cycles, ref-cycles, stalled-cycles |
| Software events | ✅ Complete | cpu-clock, task-clock, page-faults, context-switches, cpu-migrations, minor/major-faults, bpf-output |
| Cache events | ✅ Complete | L1-dcache, L1-icache, LLC, dTLB, iTLB, branch, node (all variants) |
| Raw events | ✅ Complete | rNNNN format for architecture-specific PMU events |
| Tracepoint events | ❌ Planned | syscalls, sched, irq, timer, net, etc. |
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

### Near-Term Goals (v0.2)

- **Tracepoint Support**
  - Parse tracepoint event formats from `/sys/kernel/debug/tracing/events/`
  - Support common tracepoint categories: syscalls, sched, irq, timer
  - Add `tracepoint` event type parsing and recording

- **Enhanced Event Filtering**
  - Event modifiers (`:u`, `:k`, `:p`, `:P`)
  - Filter expressions for record/report
  - Per-event privilege levels

- **Additional Commands**
  - `perf evlist`: List events in perf.data file
  - `perf diff`: Compare two perf.data files

### Mid-Term Goals (v0.3)

- **Live Monitoring Mode**
  - Real-time profiling with `perf top`
  - Live updates and statistics display
  - Interactive process selection

- **TUI Reporter**
  - Interactive terminal UI for `perf report`
  - Keyboard navigation and filtering
  - Zoom into call chains

- **Advanced Sampling**
  - LBR (Last Branch Record) support for branch profiling
  - Event groups for synchronized measurement
  - Precise sampling (PEBS) on supported hardware

### Long-Term Goals (v0.4+)

- **Extended Output Formats**
  - Flame graph SVG generation
  - Chrome tracing JSON format
  - CSV export for data analysis

- **Memory Profiling**
  - `perf mem` command for memory access patterns
  - Data address sampling
  - Memory latency analysis

- **Scheduler Analysis**
  - `perf sched` for scheduler profiling
  - Wakeup latency and migration tracking

- **Additional Architectures**
  - Evaluate demand for PowerPC, s390, MIPS support

### Not Planned

- BPF/eBPF program support (kernel subsystem, not profiling tool)
- Kernel module requirements (violates user-space design)
- Full Intel PT decoding (extensive decoder complexity)

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
