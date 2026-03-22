use crate::error::{PerfError, Result};
use crate::events::EventModifiers;
use perf_event::events::Event;
use perf_event::{Builder, Counter};

#[derive(Debug, Clone)]
pub struct PerfConfig {
    pub pid: Option<u32>,
    pub cpu: Option<u32>,
    pub include_kernel: bool,
    pub include_user: bool,
    pub include_hv: bool,
    pub inherit: bool,
    pub enable_on_exec: bool,
}

impl Default for PerfConfig {
    fn default() -> Self {
        Self {
            pid: None,
            cpu: None,
            include_kernel: false,
            include_user: true,
            include_hv: true,
            inherit: false,
            enable_on_exec: false,
        }
    }
}

impl PerfConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    pub fn with_cpu(mut self, cpu: u32) -> Self {
        self.cpu = Some(cpu);
        self
    }

    pub fn with_all_cpus(mut self) -> Self {
        self.cpu = None;
        self
    }

    pub fn with_inherit(mut self, inherit: bool) -> Self {
        self.inherit = inherit;
        self
    }

    pub fn with_include_kernel(mut self, include_kernel: bool) -> Self {
        self.include_kernel = include_kernel;
        self
    }

    pub fn with_modifiers(mut self, modifiers: EventModifiers) -> Self {
        self.include_user = !modifiers.exclude_user;
        self.include_kernel = !modifiers.exclude_kernel;
        self.include_hv = !modifiers.exclude_hv;
        self
    }
}

pub fn create_counter<E: Event + Clone + 'static>(
    event: E,
    config: &PerfConfig,
) -> Result<Counter> {
    let mut builder = Builder::new(event);

    // For system-wide monitoring (cpu specified without pid), use observe_pid(-1)
    match (&config.pid, &config.cpu) {
        (Some(pid), _) => builder.observe_pid(*pid as i32),
        (None, Some(_)) => builder.observe_pid(-1),
        (None, None) => builder.observe_self(),
    };

    if let Some(cpu) = config.cpu {
        builder.one_cpu(cpu as usize);
    } else {
        builder.any_cpu();
    }

    builder.exclude_kernel(!config.include_kernel);
    builder.exclude_hv(!config.include_hv);
    builder.exclude_user(!config.include_user);
    builder.inherit(config.inherit);

    builder.build().map_err(|e| PerfError::CounterSetup {
        source: Box::new(e),
    })
}

pub fn enable_counter(counter: &mut Counter, event_name: &str) -> Result<()> {
    counter.enable().map_err(|e| PerfError::CounterEnable {
        event_name: event_name.to_string(),
        source: Box::new(e),
    })
}

pub fn disable_counter(counter: &mut Counter, event_name: &str) -> Result<()> {
    counter.disable().map_err(|e| PerfError::CounterDisable {
        event_name: event_name.to_string(),
        source: Box::new(e),
    })
}

pub fn read_counter(counter: &mut Counter, event_name: &str) -> Result<u64> {
    counter.read().map_err(|e| PerfError::CounterRead {
        event_name: event_name.to_string(),
        source: Box::new(e),
    })
}

pub use perf_event::events::{Hardware, Software};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_cpu_sets_cpu_field() {
        let config = PerfConfig::new().with_cpu(0);
        assert_eq!(config.cpu, Some(0));
    }

    #[test]
    fn test_with_cpu_overwrites_previous_value() {
        let config = PerfConfig::new().with_cpu(0).with_cpu(3);
        assert_eq!(config.cpu, Some(3));
    }

    #[test]
    fn test_with_all_cpus_sets_cpu_to_none() {
        let config = PerfConfig::new().with_cpu(2).with_all_cpus();
        assert_eq!(config.cpu, None);
    }

    #[test]
    fn test_with_all_cpus_default_is_none() {
        let config = PerfConfig::new().with_all_cpus();
        assert_eq!(config.cpu, None);
    }

    #[test]
    fn test_builder_chaining_with_cpu() {
        let config = PerfConfig::new()
            .with_pid(1234)
            .with_cpu(1)
            .with_inherit(true);
        assert_eq!(config.pid, Some(1234));
        assert_eq!(config.cpu, Some(1));
        assert!(config.inherit);
    }
}
