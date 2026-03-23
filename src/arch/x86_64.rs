//! x86_64 (AMD64) architecture-specific PMU events and register definitions
//!
//! This module provides Intel and AMD specific performance events for x86_64 processors,
//! as well as register definitions for DWARF stack unwinding.

use super::{PmuEvent, SysfsEventDiscovery};

// ============================================================================
// x86_64 Register Definitions (matching Linux perf_regs.h)
// ============================================================================

/// x86_64 register indices matching Linux kernel perf_regs.h
///
/// These indices are used for register sampling and DWARF unwinding.
/// Reference: linux/arch/x86/include/uapi/asm/perf_regs.h
pub mod regs {
    /// RAX register (accumulator)
    pub const AX: u32 = 0;
    /// RBX register (base, callee-saved)
    pub const BX: u32 = 1;
    /// RCX register (counter)
    pub const CX: u32 = 2;
    /// RDX register (data)
    pub const DX: u32 = 3;
    /// RSI register (source index)
    pub const SI: u32 = 4;
    /// RDI register (destination index)
    pub const DI: u32 = 5;
    /// RBP register (base pointer, callee-saved, frame pointer)
    pub const BP: u32 = 6;
    /// RSP register (stack pointer)
    pub const SP: u32 = 7;
    /// RIP register (instruction pointer)
    pub const IP: u32 = 8;
    /// RFLAGS register
    pub const FLAGS: u32 = 9;
    /// CS register (code segment)
    pub const CS: u32 = 10;
    /// SS register (stack segment)
    pub const SS: u32 = 11;
    /// DS register (data segment)
    pub const DS: u32 = 12;
    /// ES register (extra segment)
    pub const ES: u32 = 13;
    /// FS register
    pub const FS: u32 = 14;
    /// GS register
    pub const GS: u32 = 15;
    /// R8 register
    pub const R8: u32 = 16;
    /// R9 register
    pub const R9: u32 = 17;
    /// R10 register
    pub const R10: u32 = 18;
    /// R11 register
    pub const R11: u32 = 19;
    /// R12 register (callee-saved)
    pub const R12: u32 = 20;
    /// R13 register (callee-saved)
    pub const R13: u32 = 21;
    /// R14 register (callee-saved)
    pub const R14: u32 = 22;
    /// R15 register (callee-saved)
    pub const R15: u32 = 23;

    /// Maximum number of x86_64 general-purpose registers
    pub const MAX: u32 = 24;
}

/// Mask for registers needed for DWARF stack unwinding.
///
/// Includes:
/// - IP (instruction pointer) - return address
/// - SP (stack pointer) - stack frame base
/// - BP (base pointer) - frame pointer
/// - BX, R12-R15 (callee-saved registers) - preserved across calls
///
/// These registers are sufficient for frame pointer and DWARF unwinding.
pub const X86_64_REGS_DWARF: u64 = (1 << regs::IP)
    | (1 << regs::SP)
    | (1 << regs::BP)
    | (1 << regs::BX)
    | (1 << regs::R12)
    | (1 << regs::R13)
    | (1 << regs::R14)
    | (1 << regs::R15);

/// Mask for callee-saved registers in x86_64 ABI.
///
/// According to the System V AMD64 ABI, these registers must be preserved
/// by the callee (function being called):
/// - RBX (base register)
/// - RBP (base pointer / frame pointer)
/// - R12, R13, R14, R15
///
/// These are critical for stack unwinding as they contain values from
/// the caller's frame.
pub const X86_64_CALLEE_SAVED: u64 = (1 << regs::BX)
    | (1 << regs::BP)
    | (1 << regs::R12)
    | (1 << regs::R13)
    | (1 << regs::R14)
    | (1 << regs::R15);

/// Returns the register mask for x86_64 DWARF unwinding.
///
/// This mask indicates which registers should be captured during sampling
/// to enable stack unwinding.
pub fn x86_64_reg_mask() -> u64 {
    X86_64_REGS_DWARF
}

/// Intel-specific PMU events
fn get_intel_events() -> Vec<PmuEvent> {
    vec![
        PmuEvent::new(
            "inst_retired.any",
            "Number of instructions retired (Precise Event)",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "cpu_clk_unhalted.thread",
            "Core cycles when the thread is not in a halt state",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "cpu_clk_unhalted.thread_any",
            "Core cycles when at least one thread is not in a halt state",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "br_inst_retired.all_branches",
            "Retired branch instructions (Precise Event)",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "br_misp_retired.all_branches",
            "Retired mispredicted branch instructions (Precise Event)",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "mem_inst_retired.any",
            "Instructions retired with memory access (Precise Event)",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "mem_retired.split_loads",
            "Retired load instructions that split across cache lines",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "ld_blocks.data_unknown",
            "Blocked loads due to unknown data address",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new("cycles_div_busy.ratio", "Cycles with any divider busy")
            .with_category("Intel hardware event"),
        PmuEvent::new(
            "fp_arith_inst_retired.scalar_double",
            "Retired scalar double-precision floating-point instructions",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "fp_arith_inst_retired.scalar_single",
            "Retired scalar single-precision floating-point instructions",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "fp_arith_inst_retired.128b_packed_double",
            "Retired 128-bit packed double-precision floating-point instructions",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "fp_arith_inst_retired.128b_packed_single",
            "Retired 128-bit packed single-precision floating-point instructions",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "fp_arith_inst_retired.256b_packed_double",
            "Retired 256-bit packed double-precision floating-point instructions",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "fp_arith_inst_retired.256b_packed_single",
            "Retired 256-bit packed single-precision floating-point instructions",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "uops_issued.any",
            "Micro-ops issued to the reservation station",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new("uops_retired.all", "Retired micro-ops")
            .with_category("Intel hardware event"),
        PmuEvent::new("uops_retired.retire_slots", "Retirement slots used")
            .with_category("Intel hardware event"),
        PmuEvent::new("l1d.retired.miss", "Retired L1D cache misses")
            .with_category("Intel hardware event"),
        PmuEvent::new("itlb_misses.walk_completed", "Completed ITLB page walks")
            .with_category("Intel hardware event"),
        PmuEvent::new(
            "dtlb_load_misses.walk_completed",
            "Completed DTLB page walks for loads",
        )
        .with_category("Intel hardware event"),
        PmuEvent::new(
            "dtlb_store_misses.walk_completed",
            "Completed DTLB page walks for stores",
        )
        .with_category("Intel hardware event"),
    ]
}

/// AMD-specific PMU events
fn get_amd_events() -> Vec<PmuEvent> {
    vec![
        PmuEvent::new("retired_instr", "Retired instructions").with_category("AMD hardware event"),
        PmuEvent::new("cycles_not_in_halt", "Cycles not in halt")
            .with_category("AMD hardware event"),
        PmuEvent::new("retired_br_instr", "Retired branch instructions")
            .with_category("AMD hardware event"),
        PmuEvent::new(
            "retired_br_misp_instr",
            "Retired mispredicted branch instructions",
        )
        .with_category("AMD hardware event"),
        PmuEvent::new("retired_mmx_fp_instr", "Retired MMX/FP instructions")
            .with_category("AMD hardware event"),
        PmuEvent::new("retired_fpu_instr", "Retired FPU instructions")
            .with_category("AMD hardware event"),
        PmuEvent::new("dispatch_stall", "Dispatch stalls").with_category("AMD hardware event"),
        PmuEvent::new("decode_stall", "Decode stalls").with_category("AMD hardware event"),
        PmuEvent::new("fp_dispatch_stall", "FP dispatch stalls")
            .with_category("AMD hardware event"),
        PmuEvent::new("ls_dispatch", "Load/store dispatches").with_category("AMD hardware event"),
        PmuEvent::new("retired_near_ret", "Retired near returns")
            .with_category("AMD hardware event"),
        PmuEvent::new(
            "retired_near_ret_mispred",
            "Retired mispredicted near returns",
        )
        .with_category("AMD hardware event"),
        PmuEvent::new("ic_fetch_stall", "Instruction cache fetch stalls")
            .with_category("AMD hardware event"),
        PmuEvent::new("ic_cache_miss", "Instruction cache misses")
            .with_category("AMD hardware event"),
        PmuEvent::new("dc_miss", "Data cache misses").with_category("AMD hardware event"),
        PmuEvent::new("tlb_miss", "TLB misses").with_category("AMD hardware event"),
        PmuEvent::new("l2_request_g1", "L2 cache requests").with_category("AMD hardware event"),
        PmuEvent::new("l2_miss", "L2 cache misses").with_category("AMD hardware event"),
    ]
}

/// Get x86_64-specific events (Intel + AMD + sysfs)
pub fn get_events() -> Vec<PmuEvent> {
    let mut events = super::get_generic_events();

    let intel_events = get_intel_events();
    let amd_events = get_amd_events();

    events.extend(intel_events);
    events.extend(amd_events);

    let discovery = SysfsEventDiscovery::new();
    let sysfs_events = discovery.discover_cpu_events();

    for sysfs_event in sysfs_events {
        if !events.iter().any(|e| e.name == sysfs_event.name) {
            events.push(sysfs_event);
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_intel_events() {
        let events = get_intel_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "inst_retired.any"));
        assert!(events.iter().any(|e| e.name == "uops_retired.all"));
    }

    #[test]
    fn test_get_amd_events() {
        let events = get_amd_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "retired_instr"));
        assert!(events.iter().any(|e| e.name == "dc_miss"));
    }

    #[test]
    fn test_get_events() {
        let events = get_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "cpu-cycles"));
        assert!(events.iter().any(|e| e.name == "inst_retired.any"));
        assert!(events.iter().any(|e| e.name == "retired_instr"));
    }

    #[test]
    fn test_x86_64_reg_mask() {
        let mask = x86_64_reg_mask();

        assert_ne!(mask & (1 << regs::IP), 0);
        assert_ne!(mask & (1 << regs::SP), 0);
        assert_ne!(mask & (1 << regs::BP), 0);
        assert_ne!(mask & (1 << regs::BX), 0);
        assert_ne!(mask & (1 << regs::R12), 0);
        assert_ne!(mask & (1 << regs::R13), 0);
        assert_ne!(mask & (1 << regs::R14), 0);
        assert_ne!(mask & (1 << regs::R15), 0);

        assert_eq!(mask & (1 << regs::AX), 0);
        assert_eq!(mask & (1 << regs::CX), 0);
        assert_eq!(mask & (1 << regs::DX), 0);
        assert_eq!(mask & (1 << regs::SI), 0);
        assert_eq!(mask & (1 << regs::DI), 0);

        assert_eq!(mask, X86_64_REGS_DWARF);
    }

    #[test]
    fn test_x86_64_callee_saved_mask() {
        let mask = X86_64_CALLEE_SAVED;

        assert_ne!(mask & (1 << regs::BX), 0);
        assert_ne!(mask & (1 << regs::BP), 0);
        assert_ne!(mask & (1 << regs::R12), 0);
        assert_ne!(mask & (1 << regs::R13), 0);
        assert_ne!(mask & (1 << regs::R14), 0);
        assert_ne!(mask & (1 << regs::R15), 0);

        assert_eq!(mask & X86_64_REGS_DWARF, mask);
    }
}
