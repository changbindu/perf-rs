//! Performance event wrapper module
//!
//! This module provides a safe wrapper around the `perf-event2` crate's Builder API
//! for creating and managing performance counters.
//!
//! # Example
//!
//! ```no_run
//! use perf_rs::core::perf_event::{create_counter, PerfConfig};
//! use perf_event::events::Hardware;
//!
//! let config = PerfConfig::default();
//! let mut counter = create_counter(Hardware::INSTRUCTIONS, &config)?;
//!
//! counter.enable()?;
//! // ... code to measure ...
//! counter.disable()?;
//!
//! let count = counter.read()?;
//! println!("Instructions: {}", count);
//! # Ok::<(), perf_rs::error::PerfError>(())
//! ```

use crate::error::{PerfError, Result};
use perf_event::events::Event;
use perf_event::{Builder, Counter, Group, GroupData};

/// Configuration options for performance counters
#[derive(Debug, Clone)]
pub struct PerfConfig {
    /// Process ID to monitor (None = current process)
    pub pid: Option<u32>,

    /// CPU to monitor (None = any CPU)
    pub cpu: Option<u32>,

    /// Whether to include kernel-space events
    pub include_kernel: bool,

    /// Whether to include user-space events
    pub include_user: bool,

    /// Whether child processes should inherit the counters
    pub inherit: bool,

    /// Whether to enable the counter immediately after creation
    pub enable_on_exec: bool,
}

impl Default for PerfConfig {
    fn default() -> Self {
        Self {
            pid: None,
            cpu: None,
            include_kernel: false,
            include_user: true,
            inherit: false,
            enable_on_exec: false,
        }
    }
}

impl PerfConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the process ID to monitor
    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    /// Set the CPU to monitor
    pub fn with_cpu(mut self, cpu: u32) -> Self {
        self.cpu = Some(cpu);
        self
    }

    /// Enable kernel-space event counting
    pub fn include_kernel(mut self, include: bool) -> Self {
        self.include_kernel = include;
        self
    }

    /// Enable user-space event counting
    pub fn include_user(mut self, include: bool) -> Self {
        self.include_user = include;
        self
    }

    /// Enable inheritance for child processes
    pub fn with_inherit(mut self, inherit: bool) -> Self {
        self.inherit = inherit;
        self
    }

    /// Enable the counter on exec
    pub fn with_enable_on_exec(mut self, enable: bool) -> Self {
        self.enable_on_exec = enable;
        self
    }
}

/// Create a single performance counter
///
/// # Arguments
///
/// * `event` - The event type to count (e.g., Hardware::INSTRUCTIONS)
/// * `config` - Configuration options for the counter
///
/// # Returns
///
/// Returns a `Counter` on success, or a `PerfError` on failure.
///
/// # Example
///
/// ```no_run
/// use perf_rs::core::perf_event::{create_counter, PerfConfig};
/// use perf_event::events::Hardware;
///
/// let config = PerfConfig::default();
/// let counter = create_counter(Hardware::CPU_CYCLES, &config)?;
/// # Ok::<(), perf_rs::error::PerfError>(())
/// ```
pub fn create_counter<E: Event + Clone + 'static>(
    event: E,
    config: &PerfConfig,
) -> Result<Counter> {
    let mut builder = Builder::new(event);

    // Configure process to monitor
    if let Some(pid) = config.pid {
        builder.observe_pid(pid as i32);
    } else {
        builder.observe_self();
    }

    // Configure CPU
    if let Some(cpu) = config.cpu {
        builder.one_cpu(cpu as usize);
    } else {
        builder.any_cpu();
    }

    // Configure kernel/user counting
    builder.exclude_kernel(!config.include_kernel);
    builder.exclude_hv(!config.include_kernel);
    builder.exclude_user(!config.include_user);

    // Configure inheritance
    builder.inherit(config.inherit);

    // Build the counter
    builder.build().map_err(|e| PerfError::CounterSetup {
        source: Box::new(e),
    })
}

/// Create a performance counter group
///
/// A group allows multiple counters to be enabled/disabled atomically
/// and ensures they measure the same time period.
///
/// # Arguments
///
/// * `config` - Configuration options for the group (pid, cpu, etc.)
///
/// # Returns
///
/// Returns a `Group` on success, or a `PerfError` on failure.
///
/// # Example
///
/// ```no_run
/// use perf_rs::core::perf_event::{create_group_with_config, add_to_group, PerfConfig};
/// use perf_event::events::Hardware;
///
/// let config = PerfConfig::new().with_pid(1234);
/// let mut group = create_group_with_config(&config)?;
/// let cycles = add_to_group(&mut group, Hardware::CPU_CYCLES, &config)?;
/// let instructions = add_to_group(&mut group, Hardware::INSTRUCTIONS, &config)?;
///
/// group.enable()?;
/// // ... code to measure ...
/// group.disable()?;
///
/// let counts = group.read()?;
/// # Ok::<(), perf_rs::error::PerfError>(())
/// ```
pub fn create_group_with_config(config: &PerfConfig) -> Result<Group> {
    let mut builder = Group::builder();

    if let Some(pid) = config.pid {
        builder.observe_pid(pid as i32);
    }

    if let Some(cpu) = config.cpu {
        builder.one_cpu(cpu as usize);
    }

    builder.build_group().map_err(|e| PerfError::CounterSetup {
        source: Box::new(e),
    })
}

/// Create a performance counter group for the current process
///
/// A group allows multiple counters to be enabled/disabled atomically
/// and ensures they measure the same time period.
///
/// # Returns
///
/// Returns a `Group` on success, or a `PerfError` on failure.
///
/// # Example
///
/// ```no_run
/// use perf_rs::core::perf_event::{create_group, add_to_group, PerfConfig};
/// use perf_event::events::Hardware;
///
/// let mut group = create_group()?;
/// let cycles = add_to_group(&mut group, Hardware::CPU_CYCLES, &PerfConfig::default())?;
/// let instructions = add_to_group(&mut group, Hardware::INSTRUCTIONS, &PerfConfig::default())?;
///
/// group.enable()?;
/// // ... code to measure ...
/// group.disable()?;
///
/// let counts = group.read()?;
/// # Ok::<(), perf_rs::error::PerfError>(())
/// ```
pub fn create_group() -> Result<Group> {
    Group::new().map_err(|e| PerfError::CounterSetup {
        source: Box::new(e),
    })
}

/// Add a counter to an existing group
///
/// # Arguments
///
/// * `group` - The group to add the counter to
/// * `event` - The event type to count
/// * `config` - Configuration options for the counter
///
/// # Returns
///
/// Returns a `Counter` on success, or a `PerfError` on failure.
///
/// # Example
///
/// ```no_run
/// use perf_rs::core::perf_event::{create_group, add_to_group, PerfConfig};
/// use perf_event::events::Hardware;
///
/// let mut group = create_group()?;
/// let counter = add_to_group(&mut group, Hardware::INSTRUCTIONS, &PerfConfig::default())?;
/// # Ok::<(), perf_rs::error::PerfError>(())
/// ```
///
/// # Note
///
/// Group members must have the same pid/cpu as the group leader (kernel requirement).
/// The `inherit` flag cannot be set on counters in a group (kernel limitation).
pub fn add_to_group<E: Event + Clone + 'static>(
    group: &mut Group,
    event: E,
    config: &PerfConfig,
) -> Result<Counter> {
    let mut builder = Builder::new(event);

    // Group members must observe the same pid/cpu as the group leader.
    // See perf-event2 docs: "any counter added to this group must observe
    // the same set of CPUs and processes as the group itself."
    if let Some(pid) = config.pid {
        builder.observe_pid(pid as i32);
    }

    if let Some(cpu) = config.cpu {
        builder.one_cpu(cpu as usize);
    }

    builder
        .exclude_kernel(!config.include_kernel)
        .exclude_hv(!config.include_kernel)
        .exclude_user(!config.include_user);
    // Note: inherit is NOT set because it's incompatible with groups (kernel limitation)

    group.add(&builder).map_err(|e| PerfError::CounterSetup {
        source: Box::new(e),
    })
}

/// Enable a counter
///
/// # Arguments
///
/// * `counter` - The counter to enable
/// * `event_name` - Name of the event for error reporting
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `PerfError` on failure.
pub fn enable_counter(counter: &mut Counter, event_name: &str) -> Result<()> {
    counter.enable().map_err(|e| PerfError::CounterEnable {
        event_name: event_name.to_string(),
        source: Box::new(e),
    })
}

/// Disable a counter
///
/// # Arguments
///
/// * `counter` - The counter to disable
/// * `event_name` - Name of the event for error reporting
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `PerfError` on failure.
pub fn disable_counter(counter: &mut Counter, event_name: &str) -> Result<()> {
    counter.disable().map_err(|e| PerfError::CounterDisable {
        event_name: event_name.to_string(),
        source: Box::new(e),
    })
}

/// Read a counter's value
///
/// # Arguments
///
/// * `counter` - The counter to read
/// * `event_name` - Name of the event for error reporting
///
/// # Returns
///
/// Returns the counter value on success, or a `PerfError` on failure.
pub fn read_counter(counter: &mut Counter, event_name: &str) -> Result<u64> {
    counter.read().map_err(|e| PerfError::CounterRead {
        event_name: event_name.to_string(),
        source: Box::new(e),
    })
}

/// Reset a counter to zero
///
/// # Arguments
///
/// * `counter` - The counter to reset
/// * `event_name` - Name of the event for error reporting
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `PerfError` on failure.
pub fn reset_counter(counter: &mut Counter, _event_name: &str) -> Result<()> {
    counter.reset().map_err(|e| PerfError::CounterSetup {
        source: Box::new(e),
    })
}

/// Enable a group of counters
///
/// # Arguments
///
/// * `group` - The group to enable
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `PerfError` on failure.
pub fn enable_group(group: &mut Group) -> Result<()> {
    group.enable().map_err(|e| PerfError::CounterEnable {
        event_name: "group".to_string(),
        source: Box::new(e),
    })
}

/// Disable a group of counters
///
/// # Arguments
///
/// * `group` - The group to disable
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `PerfError` on failure.
pub fn disable_group(group: &mut Group) -> Result<()> {
    group.disable().map_err(|e| PerfError::CounterDisable {
        event_name: "group".to_string(),
        source: Box::new(e),
    })
}

/// Read all counters in a group
///
/// # Arguments
///
/// * `group` - The group to read
///
/// # Returns
///
/// Returns `GroupData` containing all counter values on success,
/// or a `PerfError` on failure.
pub fn read_group(group: &mut Group) -> Result<GroupData> {
    group.read().map_err(|e| PerfError::CounterRead {
        event_name: "group".to_string(),
        source: Box::new(e),
    })
}

/// Reset all counters in a group to zero
///
/// # Arguments
///
/// * `group` - The group to reset
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `PerfError` on failure.
pub fn reset_group(group: &mut Group) -> Result<()> {
    group.reset().map_err(|e| PerfError::CounterSetup {
        source: Box::new(e),
    })
}

// Re-export commonly used types from perf-event2
pub use perf_event::events::{Cache, Hardware, Software};
pub use perf_event::{Counter as PerfCounter, Group as PerfGroup, GroupData as PerfGroupData};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perf_config_default() {
        let config = PerfConfig::default();
        assert!(config.pid.is_none());
        assert!(config.cpu.is_none());
        assert!(!config.include_kernel);
        assert!(config.include_user);
        assert!(!config.inherit);
        assert!(!config.enable_on_exec);
    }

    #[test]
    fn test_perf_config_builder() {
        let config = PerfConfig::new()
            .with_pid(1234)
            .with_cpu(0)
            .include_kernel(true)
            .include_user(false)
            .with_inherit(true)
            .with_enable_on_exec(true);

        assert_eq!(config.pid, Some(1234));
        assert_eq!(config.cpu, Some(0));
        assert!(config.include_kernel);
        assert!(!config.include_user);
        assert!(config.inherit);
        assert!(config.enable_on_exec);
    }

    #[test]
    fn test_create_counter_default() {
        let config = PerfConfig::default();
        let result = create_counter(Hardware::INSTRUCTIONS, &config);

        // This may fail if we don't have permissions
        match result {
            Ok(mut counter) => {
                // If we got a counter, try to use it
                let enable_result = counter.enable();
                let _ = counter.disable();
                let read_result = counter.read();

                // Just verify we can create and use the counter
                println!("Counter created successfully");
                println!("Enable result: {:?}", enable_result);
                println!("Read result: {:?}", read_result);
            }
            Err(e) => {
                // Expected if we don't have permissions
                println!(
                    "Counter creation failed (expected without permissions): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_create_group() {
        let result = create_group();

        match result {
            Ok(mut group) => {
                // Try to add a counter to the group
                let config = PerfConfig::default();
                let counter_result = add_to_group(&mut group, Hardware::CPU_CYCLES, &config);

                match counter_result {
                    Ok(_) => println!("Counter added to group successfully"),
                    Err(e) => println!("Failed to add counter to group: {}", e),
                }
            }
            Err(e) => {
                println!(
                    "Group creation failed (expected without permissions): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_counter_operations() {
        let config = PerfConfig::default();

        match create_counter(Hardware::INSTRUCTIONS, &config) {
            Ok(mut counter) => {
                // Test enable
                match enable_counter(&mut counter, "instructions") {
                    Ok(_) => println!("Counter enabled"),
                    Err(e) => println!("Enable failed: {}", e),
                }

                // Test read
                match read_counter(&mut counter, "instructions") {
                    Ok(value) => println!("Counter value: {}", value),
                    Err(e) => println!("Read failed: {}", e),
                }

                // Test disable
                match disable_counter(&mut counter, "instructions") {
                    Ok(_) => println!("Counter disabled"),
                    Err(e) => println!("Disable failed: {}", e),
                }

                // Test reset
                match reset_counter(&mut counter, "instructions") {
                    Ok(_) => println!("Counter reset"),
                    Err(e) => println!("Reset failed: {}", e),
                }
            }
            Err(e) => {
                println!(
                    "Counter creation failed (expected without permissions): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_group_operations() {
        match create_group() {
            Ok(mut group) => {
                let config = PerfConfig::default();

                // Try to add counters
                let _ = add_to_group(&mut group, Hardware::CPU_CYCLES, &config);
                let _ = add_to_group(&mut group, Hardware::INSTRUCTIONS, &config);

                // Test enable
                match enable_group(&mut group) {
                    Ok(_) => println!("Group enabled"),
                    Err(e) => println!("Group enable failed: {}", e),
                }

                // Test read
                match read_group(&mut group) {
                    Ok(data) => println!("Group data read successfully"),
                    Err(e) => println!("Group read failed: {}", e),
                }

                // Test disable
                match disable_group(&mut group) {
                    Ok(_) => println!("Group disabled"),
                    Err(e) => println!("Group disable failed: {}", e),
                }

                // Test reset
                match reset_group(&mut group) {
                    Ok(_) => println!("Group reset"),
                    Err(e) => println!("Group reset failed: {}", e),
                }
            }
            Err(e) => {
                println!(
                    "Group creation failed (expected without permissions): {}",
                    e
                );
            }
        }
    }
}
