# perf-rs Examples

This directory contains example scripts demonstrating how to use perf-rs for performance profiling.

## Example Files

### 01-list-events.sh
Demonstrates how to list available performance events on your system.
- Basic event listing
- Filtering events by name
- Showing detailed event information

### 02-stat-events.sh
Shows how to count performance events during command execution or for running processes.
- Counting specific events (CPU cycles, instructions, cache events)
- Monitoring running processes by PID
- Using event aliases

### 03-record-samples.sh
Demonstrates sample-based profiling for performance analysis.
- Recording at different sampling frequencies
- Using sampling periods
- Monitoring running processes
- Custom output files

### 04-analyze-report.sh
Shows how to analyze recorded performance data.
- Generating reports
- Sorting and filtering results
- JSON output for tooling integration

### 05-dump-trace.sh
Demonstrates exporting raw trace data from perf.data files.
- Text and JSON output formats
- Call chain visualization
- Saving traces for external analysis

### 06-complete-workflow.sh
Complete end-to-end profiling workflow example.
- Lists events
- Records samples
- Generates reports
- Exports data in multiple formats

## Usage

All scripts are executable and can be run directly:

```bash
./01-list-events.sh
```

Most examples require elevated privileges. Run with sudo:

```bash
sudo ./02-stat-events.sh
```

## Requirements

- Linux kernel 5.0 or later
- Root access or CAP_PERFMON capability
- perf-rs binary in PATH or adjust paths in scripts

## Tips

- Start with low sampling frequencies (99-999 Hz) to minimize overhead
- Use `--verbose` flag for detailed error messages
- Ensure debug symbols are present in binaries for symbol resolution
- Check `perf_event_paranoid` setting if you get permission errors