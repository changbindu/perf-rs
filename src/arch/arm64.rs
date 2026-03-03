//! ARM64 architecture PMU event definitions

use crate::arch::PmuEvent;

/// ARM64 PMU event implementation
pub struct Arm64Pmu;

impl PmuEvent for Arm64Pmu {
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
