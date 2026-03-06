use std::fs;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PrivilegeError {
    #[error("Failed to read perf_event_paranoid: {0}")]
    ParanoidReadError(#[source] io::Error),

    #[error("Invalid perf_event_paranoid value: {0}")]
    InvalidParanoidValue(String),

    #[error("Capability check failed: {0}")]
    CapabilityCheckFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeLevel {
    Full,
    Limited,
    None,
}

impl PrivilegeLevel {
    pub fn can_profile(&self) -> bool {
        matches!(self, PrivilegeLevel::Full | PrivilegeLevel::Limited)
    }

    /// Check if the privilege level allows system-wide profiling.
    ///
    /// System-wide profiling requires either:
    /// - `perf_event_paranoid <= 0`, or
    /// - `CAP_PERFMON` or `CAP_SYS_ADMIN` capability
    ///
    /// This is stricter than regular profiling permissions.
    pub fn can_profile_system_wide(&self) -> bool {
        matches!(self, PrivilegeLevel::Full)
    }

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
            ],
        }
    }
}

fn has_cap_perfmon() -> Result<bool, PrivilegeError> {
    if unsafe { libc::getuid() } == 0 {
        return Ok(true);
    }

    match fs::read_to_string("/proc/self/status") {
        Ok(status) => {
            for line in status.lines() {
                if line.starts_with("CapEff:") {
                    if let Some(hex_str) = line.strip_prefix("CapEff:") {
                        let hex_str = hex_str.trim();
                        if let Ok(caps) = u64::from_str_radix(hex_str, 16) {
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

fn has_cap_sys_admin() -> Result<bool, PrivilegeError> {
    if unsafe { libc::getuid() } == 0 {
        return Ok(true);
    }

    match fs::read_to_string("/proc/self/status") {
        Ok(status) => {
            for line in status.lines() {
                if line.starts_with("CapEff:") {
                    if let Some(hex_str) = line.strip_prefix("CapEff:") {
                        let hex_str = hex_str.trim();
                        if let Ok(caps) = u64::from_str_radix(hex_str, 16) {
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

fn read_perf_event_paranoid() -> Result<i32, PrivilegeError> {
    let content = fs::read_to_string("/proc/sys/kernel/perf_event_paranoid")
        .map_err(PrivilegeError::ParanoidReadError)?;

    let value = content
        .trim()
        .parse::<i32>()
        .map_err(|_| PrivilegeError::InvalidParanoidValue(content.trim().to_string()))?;

    Ok(value)
}

pub fn check_privilege() -> Result<PrivilegeLevel, PrivilegeError> {
    let is_root = unsafe { libc::getuid() } == 0;
    let has_perfmon = has_cap_perfmon()?;
    let has_sys_admin = has_cap_sys_admin()?;
    let paranoid = read_perf_event_paranoid()?;

    if is_root || has_perfmon || has_sys_admin {
        match paranoid {
            -1 => Ok(PrivilegeLevel::Full),
            0 | 1 => Ok(PrivilegeLevel::Full),
            2 => Ok(PrivilegeLevel::Limited),
            _ if paranoid >= 3 => Ok(PrivilegeLevel::Limited),
            _ => Ok(PrivilegeLevel::Limited),
        }
    } else {
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
    fn test_check_privilege() {
        let result = check_privilege();
        assert!(result.is_ok());
        let level = result.unwrap();
        println!("Current privilege level: {:?}", level);
    }

    #[test]
    fn test_read_perf_event_paranoid() {
        let result = read_perf_event_paranoid();
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value >= -1 && value <= 10);
    }

    #[test]
    fn test_can_profile_system_wide_full() {
        assert!(PrivilegeLevel::Full.can_profile_system_wide());
    }

    #[test]
    fn test_can_profile_system_wide_limited() {
        assert!(!PrivilegeLevel::Limited.can_profile_system_wide());
    }

    #[test]
    fn test_can_profile_system_wide_none() {
        assert!(!PrivilegeLevel::None.can_profile_system_wide());
    }
}
