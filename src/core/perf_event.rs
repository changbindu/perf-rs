use crate::error::{PerfError, Result};
use perf_event::events::Event;
use perf_event::{Builder, Counter};

#[derive(Debug, Clone)]
pub struct PerfConfig {
    pub pid: Option<u32>,
    pub cpu: Option<u32>,
    pub include_kernel: bool,
    pub include_user: bool,
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

    pub fn with_inherit(mut self, inherit: bool) -> Self {
        self.inherit = inherit;
        self
    }
}

pub fn create_counter<E: Event + Clone + 'static>(
    event: E,
    config: &PerfConfig,
) -> Result<Counter> {
    let mut builder = Builder::new(event);

    if let Some(pid) = config.pid {
        builder.observe_pid(pid as i32);
    } else {
        builder.observe_self();
    }

    if let Some(cpu) = config.cpu {
        builder.one_cpu(cpu as usize);
    } else {
        builder.any_cpu();
    }

    builder.exclude_kernel(!config.include_kernel);
    builder.exclude_hv(!config.include_kernel);
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

pub use perf_event::events::Hardware;
