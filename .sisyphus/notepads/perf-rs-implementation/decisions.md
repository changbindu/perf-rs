# Decisions - perf-rs Implementation

## [2026-03-03] Initial Setup Decisions

### Dependencies Selected
- perf-event2: Safe, high-level API for perf_event_open syscall (primary choice)
- clap: CLI argument parsing with derive macros
- thiserror: Library error types
- anyhow: Application error handling
- gimli, addr2line: DWARF debug info parsing for symbol resolution
- nix, libc, procfs: System-level operations

### Architecture Approach
- Trait-based abstraction for architecture-specific PMU events
- Defer detailed event enumeration to Task 19
- Use cfg attributes for conditional compilation

### Privilege Strategy
- Check perf_event_paranoid at startup
- Values: -1 (allow all), 0 (kernel profiling), 1 (normal), 2 (restricted)
- Graceful degradation with clear error messages