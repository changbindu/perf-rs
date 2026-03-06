//! ARM64 (AArch64) architecture-specific PMU events
//!
//! This module provides ARM Performance Monitoring Unit (PMU) events for ARM64 processors.

use super::{PmuEvent, SysfsEventDiscovery};

/// ARM Cortex-A series PMU events
fn get_cortex_events() -> Vec<PmuEvent> {
    vec![
        PmuEvent::new("sw_incr", "Software increment of performance counter")
            .with_category("ARM hardware event"),
        PmuEvent::new("l1d_cache_refill", "Level 1 data cache refill")
            .with_category("ARM hardware event"),
        PmuEvent::new("l1d_cache", "Level 1 data cache access").with_category("ARM hardware event"),
        PmuEvent::new(
            "ld_retired",
            "Instruction architecturally executed, condition code check pass, load",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new(
            "st_retired",
            "Instruction architecturally executed, condition code check pass, store",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new("inst_retired", "Instruction architecturally executed")
            .with_category("ARM hardware event"),
        PmuEvent::new("exc_taken", "Exception taken").with_category("ARM hardware event"),
        PmuEvent::new("exc_return", "Exception return").with_category("ARM hardware event"),
        PmuEvent::new(
            "cid_write_retired",
            "Instruction architecturally executed, condition code check pass, write to CONTEXTIDR",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new(
            "pc_write_retired",
            "Instruction architecturally executed, software change of the PC",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new(
            "br_immed_retired",
            "Instruction architecturally executed, immediate branch",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new(
            "br_return_retired",
            "Instruction architecturally executed, condition code check pass, return branch",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new(
            "unaligned_ldst_retired",
            "Instruction architecturally executed, unaligned load or store",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new(
            "br_mis_pred",
            "Mispredicted or not predicted branch speculatively executed",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new("cpu_cycles", "Cycle").with_category("ARM hardware event"),
        PmuEvent::new("br_pred", "Predictable branch speculatively executed")
            .with_category("ARM hardware event"),
        PmuEvent::new("mem_access", "Data memory access").with_category("ARM hardware event"),
        PmuEvent::new("l1i_cache_refill", "Level 1 instruction cache refill")
            .with_category("ARM hardware event"),
        PmuEvent::new("l1d_cache_wb", "Level 1 data cache write-back")
            .with_category("ARM hardware event"),
        PmuEvent::new("l2d_cache_refill", "Level 2 data cache refill")
            .with_category("ARM hardware event"),
        PmuEvent::new("l2d_cache", "Level 2 data cache access").with_category("ARM hardware event"),
        PmuEvent::new("l2d_cache_wb", "Level 2 data cache write-back")
            .with_category("ARM hardware event"),
        PmuEvent::new("bus_access", "Bus access").with_category("ARM hardware event"),
        PmuEvent::new("memory_error", "Local memory error").with_category("ARM hardware event"),
        PmuEvent::new("inst_spec", "Instruction speculatively executed")
            .with_category("ARM hardware event"),
        PmuEvent::new(
            "ttbr_write_retired",
            "Instruction architecturally executed, condition code check pass, write to TTBR",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new("bus_cycles", "Bus cycle").with_category("ARM hardware event"),
        PmuEvent::new(
            "l1d_cache_allocate",
            "Level 1 data cache allocation without refill",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new(
            "l2d_cache_allocate",
            "Level 2 data cache allocation without refill",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new("br_retired", "Instruction architecturally executed, branch")
            .with_category("ARM hardware event"),
        PmuEvent::new(
            "br_mis_pred_retired",
            "Instruction architecturally executed, mispredicted branch",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new(
            "stall_frontend",
            "No operation issued because of the frontend",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new(
            "stall_backend",
            "No operation issued because of the backend",
        )
        .with_category("ARM hardware event"),
        PmuEvent::new("l2d_tlb_refill", "Level 2 data TLB refill")
            .with_category("ARM hardware event"),
        PmuEvent::new("l1i_tlb_refill", "Level 1 instruction TLB refill")
            .with_category("ARM hardware event"),
        PmuEvent::new("l1d_tlb_refill", "Level 1 data TLB refill")
            .with_category("ARM hardware event"),
        PmuEvent::new("l1i_tlb", "Level 1 instruction TLB access")
            .with_category("ARM hardware event"),
        PmuEvent::new("l1d_tlb", "Level 1 data TLB access").with_category("ARM hardware event"),
        PmuEvent::new("l3d_cache_refill", "Level 3 data cache refill")
            .with_category("ARM hardware event"),
        PmuEvent::new("l3d_cache", "Level 3 data cache access").with_category("ARM hardware event"),
        PmuEvent::new("l3d_cache_wb", "Level 3 data cache write-back")
            .with_category("ARM hardware event"),
        PmuEvent::new("l2d_tlb", "Level 2 data TLB access").with_category("ARM hardware event"),
    ]
}

/// Neoverse server processor events
fn get_neoverse_events() -> Vec<PmuEvent> {
    vec![
        PmuEvent::new("ld_spec", "Instruction speculatively executed, load")
            .with_category("ARM Neoverse event"),
        PmuEvent::new("st_spec", "Instruction speculatively executed, store")
            .with_category("ARM Neoverse event"),
        PmuEvent::new(
            "dp_spec",
            "Instruction speculatively executed, integer data processing",
        )
        .with_category("ARM Neoverse event"),
        PmuEvent::new(
            "ase_spec",
            "Instruction speculatively executed, advanced SIMD",
        )
        .with_category("ARM Neoverse event"),
        PmuEvent::new(
            "vfp_spec",
            "Instruction speculatively executed, floating-point",
        )
        .with_category("ARM Neoverse event"),
        PmuEvent::new(
            "pc_write_spec",
            "Instruction speculatively executed, software change of PC",
        )
        .with_category("ARM Neoverse event"),
        PmuEvent::new(
            "crypto_spec",
            "Instruction speculatively executed, cryptographic",
        )
        .with_category("ARM Neoverse event"),
        PmuEvent::new(
            "br_imm_spec",
            "Instruction speculatively executed, immediate branch",
        )
        .with_category("ARM Neoverse event"),
        PmuEvent::new(
            "br_return_spec",
            "Instruction speculatively executed, return branch",
        )
        .with_category("ARM Neoverse event"),
        PmuEvent::new(
            "br_indirect_spec",
            "Instruction speculatively executed, indirect branch",
        )
        .with_category("ARM Neoverse event"),
        PmuEvent::new(
            "simd_advsimd_spec",
            "Instruction speculatively executed, SIMD/Advanced SIMD",
        )
        .with_category("ARM Neoverse event"),
        PmuEvent::new("simd_sve_spec", "Instruction speculatively executed, SVE")
            .with_category("ARM Neoverse event"),
    ]
}

/// Get ARM64-specific events (Cortex + Neoverse + sysfs)
pub fn get_events() -> Vec<PmuEvent> {
    let mut events = super::get_generic_events();

    let cortex_events = get_cortex_events();
    let neoverse_events = get_neoverse_events();

    events.extend(cortex_events);
    events.extend(neoverse_events);

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
    fn test_get_cortex_events() {
        let events = get_cortex_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "cpu_cycles"));
        assert!(events.iter().any(|e| e.name == "inst_retired"));
        assert!(events.iter().any(|e| e.name == "l1d_cache_refill"));
    }

    #[test]
    fn test_get_neoverse_events() {
        let events = get_neoverse_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "ld_spec"));
        assert!(events.iter().any(|e| e.name == "crypto_spec"));
    }

    #[test]
    fn test_get_events() {
        let events = get_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "cpu-cycles"));
        assert!(events.iter().any(|e| e.name == "cpu_cycles"));
        assert!(events.iter().any(|e| e.name == "inst_retired"));
    }
}
