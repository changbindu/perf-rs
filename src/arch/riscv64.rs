//! RISC-V 64-bit architecture PMU event definitions

use crate::arch::PmuEvent;

/// RISC-V 64-bit PMU event implementation
pub struct RiscV64Pmu;

impl PmuEvent for RiscV64Pmu {
    fn get_hardware_events() -> Vec<String> {
        // TODO: Implement detailed event enumeration (Task 19)
        vec![]
    }

    fn get_cache_events() -> Vec<String> {
        // TODO: Implement detailed event enumeration (Task 19)
        vec![]
    }

    fn get_raw_events() -> Vec<String> {
        // TODO: Implement detailed event enumeration (Task 19)
        vec![]
    }
}
