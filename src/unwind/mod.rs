//! Stack unwinding support using DWARF CFI information.
//!
//! This module provides functionality for stack unwinding using DWARF Call Frame
//! Information (CFI) from ELF binaries. It supports both `.eh_frame` and
//! `.eh_frame_hdr` sections for efficient unwinding.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use perf_rs::unwind::BinaryUnwindInfo;
//!
//! let path = Path::new("/usr/bin/ls");
//! let unwind_info = BinaryUnwindInfo::load(path)?;
//! # Ok::<(), perf_rs::PerfError>(())
//! ```

mod binary;
mod unwinder;

pub use binary::BinaryUnwindInfo;
pub use unwinder::{
    calculate_cfa, read_stack_u64, restore_registers, DwarfUnwinder, UserRegisters,
};
