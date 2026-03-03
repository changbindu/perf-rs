//! Architecture-specific PMU event definitions
//!
//! This module provides traits and implementations for performance monitoring
//! unit (PMU) events across different CPU architectures.

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod arm64;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;

/// Trait for architecture-specific PMU events
///
/// Each architecture implements this trait to provide access to hardware
/// performance events specific to that platform.
pub trait PmuEvent {
    /// Returns a list of available hardware events for this architecture
    ///
    /// Hardware events include CPU cycles, instructions, cache references, etc.
    /// The specific events available depend on the CPU architecture and model.
    fn get_hardware_events() -> Vec<String>;

    /// Returns a list of available cache events for this architecture
    ///
    /// Cache events track L1, L2, L3 cache hits, misses, and other cache-related
    /// performance metrics.
    fn get_cache_events() -> Vec<String>;

    /// Returns a list of raw event codes supported by this architecture
    ///
    /// Raw events allow direct access to architecture-specific performance
    /// counters using their native encoding.
    fn get_raw_events() -> Vec<String>;
}

/// Returns the architecture name as a string
pub fn get_arch_name() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "arm64"
    }
    #[cfg(target_arch = "riscv64")]
    {
        "riscv64"
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        "unknown"
    }
}
