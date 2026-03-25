use crate::error::{PerfError, Result};
use perf_event::events::Event;
use perf_event::{Builder, Counter, Record, SampleFlag, Sampler};

#[derive(Debug, Clone)]
pub struct RingBufferConfig {
    pub map_len: usize,
    pub track_lost: bool,
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        Self {
            map_len: 16,
            track_lost: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RingBufferStats {
    pub lost_samples: u64,
    pub records_read: u64,
}

pub struct RingBuffer {
    sampler: Sampler,
    stats: RingBufferStats,
}

impl RingBuffer {
    pub fn new(counter: Counter, config: RingBufferConfig) -> Result<Self> {
        let sampler = counter
            .sampled(config.map_len)
            .map_err(|e| PerfError::RingBufferSetup {
                message: format!("Failed to create sampler with map_len={}", config.map_len),
                source: Box::new(e),
            })?;

        Ok(Self {
            sampler,
            stats: RingBufferStats::default(),
        })
    }

    /// Creates a ring buffer for monitoring a specific process.
    ///
    /// # Arguments
    ///
    /// * `event` - The performance event to monitor
    /// * `pid` - The process ID to monitor
    /// * `sample_period` - The number of events between samples
    /// * `inherit` - Whether to inherit to child processes
    /// * `callchain` - Whether to enable FP callchain recording
    /// * `max_stack` - Maximum number of stack frames for callchain (0 for default)
    /// * `cpu` - Optional CPU to restrict monitoring to (for tracepoints)
    /// * `regs_user` - Optional register mask for DWARF unwinding
    /// * `stack_user` - Optional stack size for DWARF unwinding
    ///
    /// # Returns
    ///
    /// A `Result` containing the configured `RingBuffer` or an error.
    pub fn from_event_for_pid<E: Event + Clone + 'static>(
        event: E,
        pid: i32,
        sample_period: u64,
        inherit: bool,
        callchain: bool,
        max_stack: u16,
        cpu: Option<u32>,
        regs_user: Option<u64>,
        stack_user: Option<u32>,
    ) -> Result<Self> {
        let config = RingBufferConfig::default();

        let builder = &mut Builder::new(event);

        if let Some(cpu_id) = cpu {
            builder
                .observe_pid(-1)
                .one_cpu(cpu_id as usize)
                .sample_period(sample_period)
                .inherit(inherit)
                .exclude_kernel(false)
                .sample(SampleFlag::IP)
                .sample(SampleFlag::TID)
                .sample(SampleFlag::TIME)
                .sample(SampleFlag::CPU);
        } else {
            builder
                .observe_pid(pid)
                .any_cpu()
                .sample_period(sample_period)
                .inherit(inherit)
                .sample(SampleFlag::IP)
                .sample(SampleFlag::TID)
                .sample(SampleFlag::TIME);
        }

        if callchain {
            builder.sample(SampleFlag::CALLCHAIN);
            if max_stack > 0 {
                builder.sample_max_stack(max_stack);
            }
        }

        // Enable DWARF stack unwinding support
        if let (Some(regs), Some(stack)) = (regs_user, stack_user) {
            builder
                .sample(SampleFlag::REGS_USER)
                .sample(SampleFlag::STACK_USER)
                .sample_regs_user(regs)
                .sample_stack_user(stack);
        }

        let counter = builder.build().map_err(|e| PerfError::CounterSetup {
            source: Box::new(e),
        })?;

        Self::new(counter, config)
    }

    /// Creates a CPU-wide ring buffer for monitoring all processes on a specific CPU.
    ///
    /// This creates a system-wide monitoring context for the specified CPU, capturing
    /// samples from all processes running on that CPU.
    ///
    /// # Arguments
    ///
    /// * `event` - The performance event to monitor
    /// * `cpu` - The CPU ID to monitor
    /// * `sample_period` - The number of events between samples
    /// * `enable_on_exec` - Whether to enable the counter on exec (typically false for CPU-wide)
    /// * `callchain` - Whether to enable callchain (stack trace) recording
    /// * `max_stack` - Maximum number of stack frames to record (0 for default)
    /// * `regs_user` - Optional register mask for DWARF unwinding
    /// * `stack_user` - Optional stack size for DWARF unwinding
    ///
    /// # Returns
    ///
    /// A `Result` containing the configured `RingBuffer` or an error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use perf_rs::core::ringbuf::RingBuffer;
    /// use perf_event::events::Hardware;
    ///
    /// let ringbuf = RingBuffer::from_event_for_cpu(
    ///     Hardware::CPU_CYCLES,
    ///     0,  // CPU 0
    ///     100000,  // sample period
    ///     false,  // enable_on_exec
    ///     false,  // callchain
    ///     0,      // max_stack
    ///     None,   // regs_user
    ///     None,   // stack_user
    /// )?;
    /// # Ok::<(), perf_rs::error::PerfError>(())
    /// ```
    pub fn from_event_for_cpu<E: Event + Clone + 'static>(
        event: E,
        cpu: u32,
        sample_period: u64,
        enable_on_exec: bool,
        callchain: bool,
        max_stack: u16,
        regs_user: Option<u64>,
        stack_user: Option<u32>,
    ) -> Result<Self> {
        let config = RingBufferConfig::default();

        let builder = &mut Builder::new(event);
        builder
            .observe_pid(-1)
            .one_cpu(cpu as usize)
            .sample_period(sample_period)
            .enable_on_exec(enable_on_exec)
            .exclude_kernel(false)
            .sample(SampleFlag::IP)
            .sample(SampleFlag::TID)
            .sample(SampleFlag::TIME)
            .sample(SampleFlag::CPU);

        if callchain {
            builder.sample(SampleFlag::CALLCHAIN);
            if max_stack > 0 {
                builder.sample_max_stack(max_stack);
            }
        }

        // Enable DWARF stack unwinding support
        if let (Some(regs), Some(stack)) = (regs_user, stack_user) {
            builder
                .sample(SampleFlag::REGS_USER)
                .sample(SampleFlag::STACK_USER)
                .sample_regs_user(regs)
                .sample_stack_user(stack);
        }

        let counter = builder.build().map_err(|e| PerfError::CounterSetup {
            source: Box::new(e),
        })?;

        Self::new(counter, config)
    }

    pub fn enable(&mut self) -> Result<()> {
        self.sampler.enable().map_err(|e| PerfError::CounterEnable {
            event_name: "sampler".to_string(),
            source: Box::new(e),
        })
    }

    pub fn disable(&mut self) -> Result<()> {
        self.sampler
            .disable()
            .map_err(|e| PerfError::CounterDisable {
                event_name: "sampler".to_string(),
                source: Box::new(e),
            })
    }

    pub fn next_record(&mut self) -> Option<Record<'_>> {
        let record = self.sampler.next_record();
        if record.is_some() {
            self.stats.records_read += 1;
        }
        record
    }

    pub fn lost_count(&self) -> u64 {
        self.stats.lost_samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perf_event::events::Hardware;

    fn has_perf_permission() -> bool {
        if unsafe { libc::getuid() } == 0 {
            return true;
        }

        if let Ok(content) = std::fs::read_to_string("/proc/sys/kernel/perf_event_paranoid") {
            if let Ok(level) = content.trim().parse::<i32>() {
                return level <= 0;
            }
        }

        false
    }

    #[test]
    fn test_from_event_for_cpu_creates_ring_buffer() {
        if !has_perf_permission() {
            eprintln!("Skipping test: requires perf permissions");
            return;
        }

        let result = RingBuffer::from_event_for_cpu(
            Hardware::CPU_CYCLES,
            0,
            100000,
            false,
            false,
            0,
            None,
            None,
        );

        assert!(
            result.is_ok(),
            "Failed to create CPU-wide ring buffer: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_from_event_for_cpu_different_cpus() {
        if !has_perf_permission() {
            eprintln!("Skipping test: requires perf permissions");
            return;
        }

        let result = RingBuffer::from_event_for_cpu(
            Hardware::INSTRUCTIONS,
            0,
            50000,
            false,
            false,
            0,
            None,
            None,
        );
        assert!(
            result.is_ok(),
            "Failed to create ring buffer for CPU 0: {:?}",
            result.err()
        );

        let result = RingBuffer::from_event_for_cpu(
            Hardware::INSTRUCTIONS,
            1,
            50000,
            false,
            false,
            0,
            None,
            None,
        );
        if result.is_err() {
            eprintln!("CPU 1 test skipped (likely single-CPU system)");
        }
    }

    #[test]
    fn test_from_event_for_cpu_enable_disable() {
        if !has_perf_permission() {
            eprintln!("Skipping test: requires perf permissions");
            return;
        }

        let mut ringbuf = RingBuffer::from_event_for_cpu(
            Hardware::CPU_CYCLES,
            0,
            100000,
            false,
            false,
            0,
            None,
            None,
        )
        .expect("Failed to create ring buffer");

        assert!(ringbuf.enable().is_ok(), "Failed to enable ring buffer");
        assert!(ringbuf.disable().is_ok(), "Failed to disable ring buffer");
    }
}
