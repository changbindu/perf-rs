//! x86_64 (AMD64) architecture-specific PMU events
//!
//! This module provides Intel and AMD specific performance events for x86_64 processors.

use super::{PmuEvent, SysfsEventDiscovery};

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
}
