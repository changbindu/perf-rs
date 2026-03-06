//! RISC-V 64-bit architecture-specific PMU events
//!
//! This module provides RISC-V Performance Monitoring Unit events for RISC-V 64-bit processors.

use super::{PmuEvent, SysfsEventDiscovery};

/// RISC-V standard PMU events (from RISC-V privileged specification)
fn get_riscv_standard_events() -> Vec<PmuEvent> {
    vec![
        PmuEvent::new("cycles", "Total number of cycles").with_category("RISC-V hardware event"),
        PmuEvent::new("time", "Timer value").with_category("RISC-V hardware event"),
        PmuEvent::new("instret", "Number of instructions retired")
            .with_category("RISC-V hardware event"),
    ]
}

/// RISC-V common implementation events
fn get_riscv_common_events() -> Vec<PmuEvent> {
    vec![
        PmuEvent::new("l1d_read_access", "Level 1 data cache read accesses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("l1d_read_miss", "Level 1 data cache read misses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("l1d_write_access", "Level 1 data cache write accesses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("l1d_write_miss", "Level 1 data cache write misses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("l1i_read_access", "Level 1 instruction cache read accesses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("l1i_read_miss", "Level 1 instruction cache read misses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("l2_read_access", "Level 2 cache read accesses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("l2_read_miss", "Level 2 cache read misses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("l2_write_access", "Level 2 cache write accesses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("l2_write_miss", "Level 2 cache write misses")
            .with_category("RISC-V cache event"),
        PmuEvent::new("dtlb_read_miss", "Data TLB read misses").with_category("RISC-V TLB event"),
        PmuEvent::new("dtlb_write_miss", "Data TLB write misses").with_category("RISC-V TLB event"),
        PmuEvent::new("itlb_read_miss", "Instruction TLB read misses")
            .with_category("RISC-V TLB event"),
        PmuEvent::new("branch_miss", "Branch mispredictions").with_category("RISC-V branch event"),
        PmuEvent::new("branch_count", "Branch instructions executed")
            .with_category("RISC-V branch event"),
        PmuEvent::new("integer_ops", "Integer operations").with_category("RISC-V execution event"),
        PmuEvent::new("fp_ops", "Floating-point operations")
            .with_category("RISC-V execution event"),
        PmuEvent::new("mem_load", "Memory load operations").with_category("RISC-V memory event"),
        PmuEvent::new("mem_store", "Memory store operations").with_category("RISC-V memory event"),
        PmuEvent::new("mem_access", "Total memory accesses").with_category("RISC-V memory event"),
    ]
}

/// SiFive Performance Monitor events
fn get_sifive_events() -> Vec<PmuEvent> {
    vec![
        PmuEvent::new("icache_req", "Instruction cache requests").with_category("SiFive event"),
        PmuEvent::new("icache_miss", "Instruction cache misses").with_category("SiFive event"),
        PmuEvent::new("dcache_req", "Data cache requests").with_category("SiFive event"),
        PmuEvent::new("dcache_miss", "Data cache misses").with_category("SiFive event"),
        PmuEvent::new("itlb_miss", "Instruction TLB misses").with_category("SiFive event"),
        PmuEvent::new("dtlb_miss", "Data TLB misses").with_category("SiFive event"),
        PmuEvent::new("load_use_hazard", "Load-use hazards").with_category("SiFive event"),
        PmuEvent::new("control_hazard", "Control hazards").with_category("SiFive event"),
        PmuEvent::new("branch_miss", "Branch mispredictions").with_category("SiFive event"),
        PmuEvent::new("mpu_region_miss", "Memory protection unit region misses")
            .with_category("SiFive event"),
    ]
}

/// Get RISC-V 64-specific events (standard + common + vendor-specific + sysfs)
pub fn get_events() -> Vec<PmuEvent> {
    let mut events = super::get_generic_events();

    let standard_events = get_riscv_standard_events();
    let common_events = get_riscv_common_events();
    let sifive_events = get_sifive_events();

    events.extend(standard_events);
    events.extend(common_events);
    events.extend(sifive_events);

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
    fn test_get_riscv_standard_events() {
        let events = get_riscv_standard_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "cycles"));
        assert!(events.iter().any(|e| e.name == "instret"));
    }

    #[test]
    fn test_get_riscv_common_events() {
        let events = get_riscv_common_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "l1d_read_access"));
        assert!(events.iter().any(|e| e.name == "branch_miss"));
    }

    #[test]
    fn test_get_sifive_events() {
        let events = get_sifive_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "icache_miss"));
        assert!(events.iter().any(|e| e.name == "branch_miss"));
    }

    #[test]
    fn test_get_events() {
        let events = get_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "cpu-cycles"));
        assert!(events.iter().any(|e| e.name == "cycles"));
        assert!(events.iter().any(|e| e.name == "instret"));
    }
}
