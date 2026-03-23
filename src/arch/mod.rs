//! Architecture-specific PMU event support
//!
//! This module provides architecture-specific performance monitoring unit (PMU)
//! event enumeration and configuration. It supports runtime architecture detection
//! and can parse sysfs for available events on the current system.

mod arm64;
mod riscv64;
pub mod x86_64;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Architecture identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    /// x86_64 (AMD64) architecture
    X86_64,
    /// ARM64 (AArch64) architecture
    Arm64,
    /// RISC-V 64-bit architecture
    RiscV64,
    /// Unknown or unsupported architecture
    Unknown,
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::Arm64 => write!(f, "arm64"),
            Arch::RiscV64 => write!(f, "riscv64"),
            Arch::Unknown => write!(f, "unknown"),
        }
    }
}

/// PMU event information
#[derive(Debug, Clone)]
pub struct PmuEvent {
    /// Event name (e.g., "cpu-cycles")
    pub name: String,
    /// Alternative names/aliases
    pub aliases: Vec<String>,
    /// Event description
    pub description: String,
    /// Event category (Hardware, Cache, etc.)
    pub category: String,
    /// Raw event configuration (architecture-specific)
    pub config: Option<PmuEventConfig>,
    /// Whether this event was discovered from sysfs
    pub from_sysfs: bool,
}

/// Raw PMU event configuration
#[derive(Debug, Clone)]
pub struct PmuEventConfig {
    /// Event number
    pub event: u64,
    /// Unit mask (umask)
    pub umask: Option<u64>,
    /// Additional configuration flags
    pub cmask: Option<u64>,
    /// Any flag
    pub any: Option<u64>,
    /// Edge detect
    pub edge: Option<u64>,
    /// Invert counter mask
    pub inv: Option<u64>,
    /// Load latency threshold
    pub ldlat: Option<u64>,
}

impl PmuEvent {
    /// Create a new PMU event
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            description: description.into(),
            category: "Hardware event".to_string(),
            config: None,
            from_sysfs: false,
        }
    }

    /// Add an alias to the event
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    /// Set the event category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// Set the raw event configuration
    pub fn with_config(mut self, config: PmuEventConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Mark as discovered from sysfs
    pub fn from_sysfs(mut self) -> Self {
        self.from_sysfs = true;
        self
    }
}

/// Detect the current architecture at runtime
pub fn detect_arch() -> Arch {
    // Use compile-time detection first (more reliable)
    #[cfg(target_arch = "x86_64")]
    {
        Arch::X86_64
    }

    #[cfg(target_arch = "aarch64")]
    {
        Arch::Arm64
    }

    #[cfg(target_arch = "riscv64")]
    {
        Arch::RiscV64
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        // Fallback to runtime detection via uname
        runtime_detect_arch()
    }
}

/// Runtime architecture detection via uname
fn runtime_detect_arch() -> Arch {
    use std::process::Command;

    if let Ok(output) = Command::new("uname").arg("-m").output() {
        if let Ok(arch_str) = String::from_utf8(output.stdout) {
            let arch_str = arch_str.trim();
            match arch_str {
                "x86_64" | "amd64" => return Arch::X86_64,
                "aarch64" | "arm64" => return Arch::Arm64,
                "riscv64" => return Arch::RiscV64,
                _ => {}
            }
        }
    }

    Arch::Unknown
}

/// Sysfs event discovery
pub struct SysfsEventDiscovery {
    /// Path to sysfs event sources
    sysfs_path: PathBuf,
}

impl SysfsEventDiscovery {
    /// Create a new sysfs event discovery instance
    pub fn new() -> Self {
        Self {
            sysfs_path: PathBuf::from("/sys/bus/event_source/devices"),
        }
    }

    /// Create with custom sysfs path (for testing)
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            sysfs_path: path.into(),
        }
    }

    /// Get available PMU devices
    pub fn get_pmu_devices(&self) -> Vec<String> {
        match fs::read_dir(&self.sysfs_path) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Discover events from a specific PMU device
    pub fn discover_events(&self, device: &str) -> Vec<PmuEvent> {
        let events_path = self.sysfs_path.join(device).join("events");
        let mut events = Vec::new();

        if let Ok(entries) = fs::read_dir(events_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let file_name = entry.file_name();
                let event_name = file_name.to_string_lossy();

                if let Some(event) = self.parse_event_file(device, &event_name, &entry.path()) {
                    events.push(event);
                }
            }
        }

        events
    }

    /// Parse a single event file from sysfs
    fn parse_event_file(
        &self,
        device: &str,
        event_name: &str,
        event_path: &Path,
    ) -> Option<PmuEvent> {
        let content = fs::read_to_string(event_path).ok()?;
        let config = self.parse_event_config(&content)?;

        let description = format!("{} PMU event: {}", device, event_name);

        Some(
            PmuEvent::new(event_name, description)
                .with_category(format!("{} event", device))
                .with_config(config)
                .from_sysfs(),
        )
    }

    /// Parse event configuration from sysfs format
    /// Format: event=0xXX[,umask=0xXX][,cmask=0xXX][,any=N][,edge=N][,inv=N][,ldlat=N]
    fn parse_event_config(&self, content: &str) -> Option<PmuEventConfig> {
        let mut config = PmuEventConfig {
            event: 0,
            umask: None,
            cmask: None,
            any: None,
            edge: None,
            inv: None,
            ldlat: None,
        };

        for part in content.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                let parsed_value = if value.starts_with("0x") {
                    u64::from_str_radix(value.trim_start_matches("0x"), 16).ok()?
                } else {
                    value.parse().ok()?
                };

                match key {
                    "event" => config.event = parsed_value,
                    "umask" => config.umask = Some(parsed_value),
                    "cmask" => config.cmask = Some(parsed_value),
                    "any" => config.any = Some(parsed_value),
                    "edge" => config.edge = Some(parsed_value),
                    "inv" => config.inv = Some(parsed_value),
                    "ldlat" => config.ldlat = Some(parsed_value),
                    _ => {}
                }
            }
        }

        Some(config)
    }

    /// Discover all CPU events
    pub fn discover_cpu_events(&self) -> Vec<PmuEvent> {
        self.discover_events("cpu")
    }
}

impl Default for SysfsEventDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Get architecture-specific events
pub fn get_arch_events() -> Vec<PmuEvent> {
    let arch = detect_arch();

    match arch {
        Arch::X86_64 => get_x86_64_events(),
        Arch::Arm64 => get_arm64_events(),
        Arch::RiscV64 => get_riscv64_events(),
        Arch::Unknown => get_generic_events(),
    }
}

/// Get generic events (fallback for unknown architectures)
pub fn get_generic_events() -> Vec<PmuEvent> {
    vec![
        PmuEvent::new("cpu-cycles", "Total cycles")
            .with_alias("cycles")
            .with_category("Hardware event"),
        PmuEvent::new("instructions", "Retired instructions").with_category("Hardware event"),
        PmuEvent::new("cache-references", "Cache accesses").with_category("Hardware event"),
        PmuEvent::new("cache-misses", "Cache misses").with_category("Hardware event"),
        PmuEvent::new("branch-instructions", "Retired branch instructions")
            .with_alias("branches")
            .with_category("Hardware event"),
        PmuEvent::new("branch-misses", "Mispredicted branch instructions")
            .with_category("Hardware event"),
        PmuEvent::new("bus-cycles", "Bus cycles").with_category("Hardware event"),
        PmuEvent::new(
            "ref-cycles",
            "Total cycles (independent of frequency scaling)",
        )
        .with_category("Hardware event"),
    ]
}

/// Get x86_64-specific events
fn get_x86_64_events() -> Vec<PmuEvent> {
    x86_64::get_events()
}

/// Get ARM64-specific events
fn get_arm64_events() -> Vec<PmuEvent> {
    arm64::get_events()
}

/// Get RISC-V 64-specific events
fn get_riscv64_events() -> Vec<PmuEvent> {
    riscv64::get_events()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_arch() {
        let arch = detect_arch();
        // On x86_64 systems, should detect X86_64
        #[cfg(target_arch = "x86_64")]
        assert_eq!(arch, Arch::X86_64);

        #[cfg(target_arch = "aarch64")]
        assert_eq!(arch, Arch::Arm64);

        #[cfg(target_arch = "riscv64")]
        assert_eq!(arch, Arch::RiscV64);
    }

    #[test]
    fn test_arch_display() {
        assert_eq!(format!("{}", Arch::X86_64), "x86_64");
        assert_eq!(format!("{}", Arch::Arm64), "arm64");
        assert_eq!(format!("{}", Arch::RiscV64), "riscv64");
        assert_eq!(format!("{}", Arch::Unknown), "unknown");
    }

    #[test]
    fn test_pmu_event_creation() {
        let event = PmuEvent::new("cpu-cycles", "Total cycles")
            .with_alias("cycles")
            .with_category("Hardware event");

        assert_eq!(event.name, "cpu-cycles");
        assert_eq!(event.description, "Total cycles");
        assert_eq!(event.aliases, vec!["cycles"]);
        assert_eq!(event.category, "Hardware event");
        assert!(!event.from_sysfs);
    }

    #[test]
    fn test_pmu_event_config() {
        let config = PmuEventConfig {
            event: 0xc4,
            umask: Some(0x00),
            cmask: None,
            any: None,
            edge: None,
            inv: None,
            ldlat: None,
        };

        let event = PmuEvent::new("test-event", "Test").with_config(config);
        assert!(event.config.is_some());
        assert_eq!(event.config.unwrap().event, 0xc4);
    }

    #[test]
    fn test_get_generic_events() {
        let events = get_generic_events();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.name == "cpu-cycles"));
        assert!(events.iter().any(|e| e.name == "instructions"));
    }

    #[test]
    fn test_get_arch_events() {
        let events = get_arch_events();
        assert!(!events.is_empty());
        // Should contain at least generic events
        assert!(events.iter().any(|e| e.name == "cpu-cycles"));
    }

    #[test]
    fn test_sysfs_discovery_creation() {
        let discovery = SysfsEventDiscovery::new();
        assert_eq!(
            discovery.sysfs_path,
            PathBuf::from("/sys/bus/event_source/devices")
        );
    }

    #[test]
    fn test_sysfs_discovery_custom_path() {
        let discovery = SysfsEventDiscovery::with_path("/tmp/test");
        assert_eq!(discovery.sysfs_path, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_parse_event_config() {
        let discovery = SysfsEventDiscovery::new();

        // Test simple event format
        let config = discovery.parse_event_config("event=0xc4");
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.event, 0xc4);
        assert!(config.umask.is_none());

        // Test with umask
        let config = discovery.parse_event_config("event=0x3c,umask=0x01");
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.event, 0x3c);
        assert_eq!(config.umask, Some(0x01));

        // Test with multiple fields
        let config = discovery.parse_event_config("event=0xd,umask=0x1,cmask=1,any=12");
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.event, 0xd);
        assert_eq!(config.umask, Some(0x1));
        assert_eq!(config.cmask, Some(1));
        assert_eq!(config.any, Some(12));
    }
}
