//! Privilege checking for perf operations
//!
//! This module provides privilege level detection and capability checking
//! for Linux performance monitoring operations.

use std::fs;
use std::io;
use thiserror::Error;

/// Errors that can occur during privilege checking
#[derive(Error, Debug)]
pub enum PrivilegeError {
    #[error("Failed to read perf_event_paranoid: {0}")]
    ParanoidReadError(#[source] io::Error),

    #[error("Invalid perf_event_paranoid value: {0}")]
    InvalidParanoidValue(String),

    #[error("Insufficient privileges for operation: {0}")]
    InsufficientPrivileges(String),

    #[error("Capability check failed: {0}")]
    CapabilityCheckFailed(String),
}

/// Privilege levels for perf operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeLevel {
    /// Full access - can perform all perf operations
    /// Either root or perf_event_paranoid <= 1 with CAP_PERFMON
    Full,

    /// Limited access - can perform some perf operations
    /// perf_event_paranoid = 2 or non-root with limited capabilities
    Limited,

    /// No access - cannot perform perf operations
    /// perf_event_paranoid >= 3 or no capabilities
    None,
}

impl PrivilegeLevel {
    /// Returns true if this level allows the operation
    pub fn can_profile(&self) -> bool {
        matches!(self, PrivilegeLevel::Full | PrivilegeLevel::Limited)
    }

    /// Returns true if this level allows kernel profiling
    pub fn can_profile_kernel(&self) -> bool {
        matches!(self, PrivilegeLevel::Full)
    }

    /// Returns a human-readable description of the privilege level
    pub fn description(&self) -> &'static str {
        match self {
            PrivilegeLevel::Full => "Full perf access - can profile kernel and user space",
            PrivilegeLevel::Limited => "Limited perf access - can profile user space only",
            PrivilegeLevel::None => "No perf access - insufficient privileges",
        }
    }

    /// Returns suggestions for improving privilege level
    pub fn suggestions(&self) -> Vec<String> {
        match self {
            PrivilegeLevel::Full => {
                vec!["You have full perf access. No action needed.".to_string()]
            }
            PrivilegeLevel::Limited => vec![
                "To enable kernel profiling, run with sudo or adjust perf_event_paranoid:"
                    .to_string(),
                "  sudo sysctl -w kernel.perf_event_paranoid=1".to_string(),
                "Or add CAP_PERFMON capability:".to_string(),
                "  sudo setcap cap_perfmon+ep /path/to/perf-rs".to_string(),
            ],
            PrivilegeLevel::None => vec![
                "To enable perf access, choose one of the following:".to_string(),
                "1. Run with sudo:".to_string(),
                "   sudo perf-rs ...".to_string(),
                "2. Adjust perf_event_paranoid (requires root):".to_string(),
                "   sudo sysctl -w kernel.perf_event_paranoid=2".to_string(),
                "3. Add CAP_PERFMON capability:".to_string(),
                "   sudo setcap cap_perfmon+ep /path/to/perf-rs".to_string(),
                "4. Add CAP_SYS_ADMIN capability (less secure):".to_string(),
                "   sudo setcap cap_sys_admin+ep /path/to/perf-rs".to_string(),
            ],
        }
    }
}

/// Check if the process has CAP_PERFMON capability
fn has_cap_perfmon() -> Result<bool, PrivilegeError> {
    // Check if we're running as root (has all capabilities)
    if unsafe { libc::getuid() } == 0 {
        return Ok(true);
    }

    // Try to check capabilities using /proc/self/status
    match fs::read_to_string("/proc/self/status") {
        Ok(status) => {
            for line in status.lines() {
                if line.starts_with("CapEff:") {
                    // Parse the effective capabilities bitmask
                    if let Some(hex_str) = line.strip_prefix("CapEff:") {
                        let hex_str = hex_str.trim();
                        if let Ok(caps) = u64::from_str_radix(hex_str, 16) {
                            // CAP_PERFMON is capability 38 (bit 38)
                            const CAP_PERFMON: u64 = 1 << 38;
                            return Ok((caps & CAP_PERFMON) != 0);
                        }
                    }
                }
            }
            Ok(false)
        }
        Err(e) => Err(PrivilegeError::CapabilityCheckFailed(format!(
            "Failed to read /proc/self/status: {}",
            e
        ))),
    }
}

/// Check if the process has CAP_SYS_ADMIN capability
fn has_cap_sys_admin() -> Result<bool, PrivilegeError> {
    // Check if we're running as root (has all capabilities)
    if unsafe { libc::getuid() } == 0 {
        return Ok(true);
    }

    // Try to check capabilities using /proc/self/status
    match fs::read_to_string("/proc/self/status") {
        Ok(status) => {
            for line in status.lines() {
                if line.starts_with("CapEff:") {
                    // Parse the effective capabilities bitmask
                    if let Some(hex_str) = line.strip_prefix("CapEff:") {
                        let hex_str = hex_str.trim();
                        if let Ok(caps) = u64::from_str_radix(hex_str, 16) {
                            // CAP_SYS_ADMIN is capability 21 (bit 21)
                            const CAP_SYS_ADMIN: u64 = 1 << 21;
                            return Ok((caps & CAP_SYS_ADMIN) != 0);
                        }
                    }
                }
            }
            Ok(false)
        }
        Err(e) => Err(PrivilegeError::CapabilityCheckFailed(format!(
            "Failed to read /proc/self/status: {}",
            e
        ))),
    }
}

/// Read the perf_event_paranoid value from kernel
fn read_perf_event_paranoid() -> Result<i32, PrivilegeError> {
    let content = fs::read_to_string("/proc/sys/kernel/perf_event_paranoid")
        .map_err(PrivilegeError::ParanoidReadError)?;

    let value = content
        .trim()
        .parse::<i32>()
        .map_err(|_| PrivilegeError::InvalidParanoidValue(content.trim().to_string()))?;

    Ok(value)
}

/// Check the current privilege level for perf operations
///
/// This function examines:
/// - The value of /proc/sys/kernel/perf_event_paranoid
/// - Whether the process has CAP_PERFMON capability
/// - Whether the process has CAP_SYS_ADMIN capability
/// - Whether the process is running as root
///
/// # Returns
///
/// Returns the detected PrivilegeLevel or an error if the check fails.
///
/// # Example
///
/// ```no_run
/// use perf_rs::core::privilege::{check_privilege, PrivilegeLevel};
///
/// match check_privilege() {
///     Ok(level) => {
///         println!("Privilege level: {:?}", level);
///         if !level.can_profile() {
///             for suggestion in level.suggestions() {
///                 println!("{}", suggestion);
///             }
///         }
///     }
///     Err(e) => eprintln!("Failed to check privileges: {}", e),
/// }
/// ```
pub fn check_privilege() -> Result<PrivilegeLevel, PrivilegeError> {
    // Check if running as root
    let is_root = unsafe { libc::getuid() } == 0;

    // Check capabilities
    let has_perfmon = has_cap_perfmon()?;
    let has_sys_admin = has_cap_sys_admin()?;

    // Read perf_event_paranoid value
    let paranoid = read_perf_event_paranoid()?;

    // Determine privilege level based on paranoid value and capabilities
    // perf_event_paranoid values:
    // -1: Allow all users (no restrictions)
    //  0: Allow kernel profiling for CAP_SYS_ADMIN
    //  1: Allow kernel profiling for normal users (with CAP_PERFMON)
    //  2: Disallow kernel profiling for normal users
    //  3+: Disallow CPU event access for normal users
    //  4+: Disallow any perf access for normal users

    if is_root || has_perfmon || has_sys_admin {
        // Root or with capabilities - check paranoid value
        match paranoid {
            -1 => Ok(PrivilegeLevel::Full),
            0 | 1 => Ok(PrivilegeLevel::Full),
            2 => Ok(PrivilegeLevel::Limited),
            _ if paranoid >= 3 => Ok(PrivilegeLevel::Limited),
            _ => Ok(PrivilegeLevel::Limited),
        }
    } else {
        // Normal user without capabilities
        match paranoid {
            -1 => Ok(PrivilegeLevel::Full),
            0 | 1 => Ok(PrivilegeLevel::Limited),
            2 => Ok(PrivilegeLevel::Limited),
            3 => Ok(PrivilegeLevel::Limited),
            _ if paranoid >= 4 => Ok(PrivilegeLevel::None),
            _ => Ok(PrivilegeLevel::Limited),
        }
    }
}

/// Ensure the process has sufficient privileges for the requested operation
///
/// # Arguments
///
/// * `require_kernel` - If true, require privileges for kernel profiling
///
/// # Returns
///
/// Returns Ok(()) if privileges are sufficient, or an error with suggestions.
///
/// # Example
///
/// ```no_run
/// use perf_rs::core::privilege::ensure_privilege;
///
/// // Check for user-space profiling
/// if let Err(e) = ensure_privilege(false) {
///     eprintln!("Error: {}", e);
///     std::process::exit(1);
/// }
///
/// // Check for kernel profiling
/// if let Err(e) = ensure_privilege(true) {
///     eprintln!("Error: {}", e);
///     std::process::exit(1);
/// }
/// ```
pub fn ensure_privilege(require_kernel: bool) -> Result<(), PrivilegeError> {
    let level = check_privilege()?;

    if require_kernel && !level.can_profile_kernel() {
        let suggestions = level.suggestions().join("\n");
        return Err(PrivilegeError::InsufficientPrivileges(format!(
            "Kernel profiling requires elevated privileges.\n{}",
            suggestions
        )));
    }

    if !require_kernel && !level.can_profile() {
        let suggestions = level.suggestions().join("\n");
        return Err(PrivilegeError::InsufficientPrivileges(format!(
            "Profiling requires elevated privileges.\n{}",
            suggestions
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_level_can_profile() {
        assert!(PrivilegeLevel::Full.can_profile());
        assert!(PrivilegeLevel::Limited.can_profile());
        assert!(!PrivilegeLevel::None.can_profile());
    }

    #[test]
    fn test_privilege_level_can_profile_kernel() {
        assert!(PrivilegeLevel::Full.can_profile_kernel());
        assert!(!PrivilegeLevel::Limited.can_profile_kernel());
        assert!(!PrivilegeLevel::None.can_profile_kernel());
    }

    #[test]
    fn test_privilege_level_description() {
        assert!(!PrivilegeLevel::Full.description().is_empty());
        assert!(!PrivilegeLevel::Limited.description().is_empty());
        assert!(!PrivilegeLevel::None.description().is_empty());
    }

    #[test]
    fn test_privilege_level_suggestions() {
        assert!(!PrivilegeLevel::Full.suggestions().is_empty());
        assert!(!PrivilegeLevel::Limited.suggestions().is_empty());
        assert!(!PrivilegeLevel::None.suggestions().is_empty());
    }

    #[test]
    fn test_check_privilege() {
        let result = check_privilege();
        assert!(result.is_ok());

        let level = result.unwrap();
        println!("Current privilege level: {:?}", level);
        println!("Description: {}", level.description());

        for suggestion in level.suggestions() {
            println!("Suggestion: {}", suggestion);
        }
    }

    #[test]
    fn test_read_perf_event_paranoid() {
        let result = read_perf_event_paranoid();
        assert!(result.is_ok());

        let value = result.unwrap();
        println!("perf_event_paranoid value: {}", value);

        assert!(value >= -1 && value <= 10);
    }

    #[test]
    fn test_capability_checks() {
        let perfmon = has_cap_perfmon();
        let sys_admin = has_cap_sys_admin();

        println!("CAP_PERFMON: {:?}", perfmon);
        println!("CAP_SYS_ADMIN: {:?}", sys_admin);

        assert!(perfmon.is_ok());
        assert!(sys_admin.is_ok());
    }

    #[test]
    fn test_ensure_privilege_user_space() {
        let result = ensure_privilege(false);
        println!("ensure_privilege(false): {:?}", result);
        let _ = result;
    }

    #[test]
    fn test_ensure_privilege_kernel() {
        let result = ensure_privilege(true);
        println!("ensure_privilege(true): {:?}", result);
        let _ = result;
    }
}
