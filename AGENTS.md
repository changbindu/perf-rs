# AGENTS.md - perf-rs Development Guide

This document provides essential information for agentic coding agents working in this repository.

## Project Overview

**perf-rs** is a Linux performance monitoring tool written in Rust. It provides functionality similar to the `perf` command-line tool, including:
- Listing available performance events
- Counting performance events (stat command)
- Recording samples for profiling (record command)
- Symbol resolution from ELF binaries and kernel symbols

**Minimum Rust Version:** 1.70
**Edition:** 2021

## Build/Lint/Test Commands

```bash
# Build the project
cargo build

# Build in release mode (optimized)
cargo build --release

# Check for compilation errors without building
cargo check

# Run all tests
cargo test

# Run a specific test by name
cargo test test_name

# Run tests in a specific file
cargo test --test filename

# Run tests with output shown
cargo test -- --nocapture

# Run linting with clippy
cargo clippy

# Auto-fix clippy warnings
cargo clippy --fix

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Generate documentation
cargo doc --open

# Run the application (requires root or CAP_SYS_ADMIN for perf events)
cargo run -- [args]

# Example: Run with a command
cargo run -- stat -- ls -la
cargo run -- record --frequency 99 -- ./target/debug/myprogram
```

## Code Style Guidelines

### Imports Organization

Imports are organized in groups, separated by blank lines:
1. External crates (std first, then third-party)
2. Internal crate imports (`use crate::`)

```rust
use std::path::PathBuf;
use thiserror::Error;

use crate::error::Result;
```

### Documentation

- Module-level docs use `//!` comments
- Function/struct docs use `///` comments
- Include examples in code blocks when applicable
- Use `# Ok::<(), Error>()` at the end of example blocks for error handling

```rust
//! Module description here.

/// Brief description of function.
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value.
///
/// # Example
///
/// ```no_run
/// let result = my_function(arg)?;
/// # Ok::<(), MyError>(())
/// ```
pub fn my_function(param: &str) -> Result<()> { ... }
```

### Error Handling

This project uses a custom error type (`PerfError`) with `thiserror`:

```rust
use crate::error::{PerfError, Result};

// Create specific error variants
return Err(PerfError::FileNotFound {
    path: PathBuf::from("/some/path"),
});

// Use .map_err() for context
file.read().map_err(|e| PerfError::FileRead {
    path: path.to_path_buf(),
    source: Box::new(e),
})?;

// Use anyhow::Context for additional error context
some_operation().with_context(|| "Failed to do X")?;
```

**Error variant naming convention:**
- Use descriptive names: `ProcessAttach`, `CounterSetup`, `FileNotFound`
- Include relevant fields for error messages (e.g., `pid`, `path`, `event_name`)
- Always include `source` field for chaining errors

### Naming Conventions

- **Functions/Variables**: `snake_case` (e.g., `create_counter`, `event_name`)
- **Types/Enums/Traits**: `PascalCase` (e.g., `PerfConfig`, `SymbolResolver`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_BUFFER_SIZE`)
- **Modules**: `snake_case` (e.g., `perf_event`, `ringbuf`)
- **Test functions**: `test_` prefix (e.g., `test_parse_event`)

### Struct Design

Use the builder pattern with `with_*` methods for configuration:

```rust
pub struct Config {
    pub pid: Option<u32>,
    pub cpu: Option<u32>,
}

impl Config {
    pub fn new() -> Self { Self::default() }

    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    pub fn with_cpu(mut self, cpu: u32) -> Self {
        self.cpu = Some(cpu);
        self
    }
}
```

Always implement `Default` trait when applicable.

### Testing

- Place unit tests in `#[cfg(test)] mod tests` within the same file
- Use descriptive test names: `test_<what>_<condition>_<expected>`
- Use `assert!`, `assert_eq!`, `assert_matches!` for assertions

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event_valid_input() {
        let result = parse_event("cpu-cycles");
        assert!(matches!(result, Ok(Hardware::CPU_CYCLES)));
    }

    #[test]
    fn test_parse_event_invalid_returns_error() {
        let result = parse_event("unknown-event");
        assert!(result.is_err());
    }
}
```

### Module Structure

- Use `mod.rs` for re-exporting public API
- Keep modules focused on a single responsibility
- Re-export commonly used types at module level

```rust
// src/core/mod.rs
pub mod perf_data;
pub mod perf_event;
pub mod privilege;
pub mod ringbuf;

// Re-export for convenience
pub use perf_event::{PerfConfig, Hardware};
```

## Project Structure

```
src/
├── main.rs           # Entry point, command dispatch
├── cli.rs            # CLI definitions using clap
├── error.rs          # Error types (PerfError)
├── arch/             # Architecture-specific code
│   ├── mod.rs
│   ├── x86_64.rs
│   ├── arm64.rs
│   └── riscv64.rs
├── commands/         # Subcommand implementations
│   ├── mod.rs
│   ├── list.rs       # perf list
│   ├── stat.rs       # perf stat
│   └── record.rs     # perf record
├── core/             # Core functionality
│   ├── mod.rs
│   ├── perf_event.rs # Performance counter API
│   ├── perf_data.rs  # perf.data file handling
│   ├── ringbuf.rs    # Ring buffer for sampling
│   └── privilege.rs  # Privilege checking
└── symbols/          # Symbol resolution
    ├── mod.rs
    ├── elf.rs        # ELF symbol resolver
    └── kernel.rs     # Kernel symbol resolver
```

## Key Dependencies

- **clap** (derive): CLI argument parsing
- **perf-event2**: Linux perf event API wrapper
- **thiserror**: Custom error types
- **anyhow**: Error handling with context
- **nix**: Unix system calls (fork, exec, signals)
- **procfs**: Reading /proc filesystem
- **addr2line/gimli**: DWARF debug info parsing
- **memmap2**: Memory-mapped file I/O

## Important Notes

### Privilege Requirements

Performance monitoring requires elevated privileges:
- Root user, or
- `CAP_SYS_ADMIN` capability, or
- `perf_event_paranoid` sysctl set appropriately

Always check privileges using `check_privilege()` before attempting perf operations.

### Safety

This project uses `unsafe` blocks for:
- `fork()` system call
- Memory-mapped I/O (`memmap2`)
- Direct syscall wrappers

All unsafe blocks should have safety comments explaining the invariants.

### Code Guidelines

1. **Never** use `unwrap()` in production code - use proper error handling
2. **Never** suppress errors with `ok()` unless explicitly ignoring is intended
3. **Always** check for permission/privilege before perf operations
4. **Always** clean up resources (close files, disable counters)
5. Use `?` operator for error propagation
6. Prefer `Result<T>` over panicking

### Git Conventions

- Write descriptive commit messages
- Keep commits atomic (one logical change per commit)
- Run `cargo fmt` and `cargo clippy` before committing