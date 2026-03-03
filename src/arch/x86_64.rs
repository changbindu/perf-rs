//! x86_64 architecture PMU event definitions

use crate::arch::PmuEvent;

/// x86_64 PMU event implementation
pub struct X86_64Pmu;

impl PmuEvent for X86_64Pmu {
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
