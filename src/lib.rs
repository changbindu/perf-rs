//! Linux performance monitoring library in Rust
//!
//! This library provides functionality for Linux performance monitoring,
//! similar to the `perf` command-line tool.

pub mod arch;
pub mod cli;
pub mod commands;
pub mod core;
pub mod error;
pub mod events;
pub mod pager;
pub mod symbols;
pub mod tracepoint;

pub use error::{PerfError, Result};
