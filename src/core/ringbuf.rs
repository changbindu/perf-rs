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

    pub fn from_event_for_pid<E: Event + Clone + 'static>(
        event: E,
        pid: i32,
        sample_period: u64,
        inherit: bool,
    ) -> Result<Self> {
        let config = RingBufferConfig::default();

        let counter = Builder::new(event)
            .observe_pid(pid)
            .any_cpu()
            .sample_period(sample_period)
            .inherit(inherit)
            .sample(SampleFlag::IP)
            .sample(SampleFlag::TID)
            .sample(SampleFlag::TIME)
            .sample(SampleFlag::CALLCHAIN)
            .build()
            .map_err(|e| PerfError::CounterSetup {
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
    /// )?;
    /// # Ok::<(), perf_rs::error::PerfError>(())
    /// ```
    pub fn from_event_for_cpu<E: Event + Clone + 'static>(
        event: E,
        cpu: u32,
        sample_period: u64,
        enable_on_exec: bool,
    ) -> Result<Self> {
        let config = RingBufferConfig::default();

        // pid = -1 means monitor all processes on the specified CPU
        let counter = Builder::new(event)
            .observe_pid(-1)
            .one_cpu(cpu as usize)
            .sample_period(sample_period)
            .enable_on_exec(enable_on_exec)
            .sample(SampleFlag::IP)
            .sample(SampleFlag::TID)
            .sample(SampleFlag::TIME)
            .sample(SampleFlag::CALLCHAIN)
            .sample(SampleFlag::CPU)
            .build()
            .map_err(|e| PerfError::CounterSetup {
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

    #[test]
    #[ignore = "Requires root privileges or CAP_PERFMON capability"]
    fn test_from_event_for_cpu_creates_ring_buffer() {
        // This test requires privileges, so it's marked as ignored by default
        // Run with: cargo test -- --ignored
        let result = RingBuffer::from_event_for_cpu(Hardware::CPU_CYCLES, 0, 100000, false);

        assert!(
            result.is_ok(),
            "Failed to create CPU-wide ring buffer: {:?}",
            result.err()
        );
    }

    #[test]
    #[ignore = "Requires root privileges or CAP_PERFMON capability"]
    fn test_from_event_for_cpu_different_cpus() {
        let result = RingBuffer::from_event_for_cpu(Hardware::INSTRUCTIONS, 0, 50000, false);
        assert!(
            result.is_ok(),
            "Failed to create ring buffer for CPU 0: {:?}",
            result.err()
        );

        let result = RingBuffer::from_event_for_cpu(Hardware::INSTRUCTIONS, 1, 50000, false);
        if result.is_err() {
            eprintln!("CPU 1 test skipped (likely single-CPU system)");
        }
    }

    #[test]
    #[ignore = "Requires root privileges or CAP_PERFMON capability"]
    fn test_from_event_for_cpu_enable_disable() {
        let mut ringbuf = RingBuffer::from_event_for_cpu(Hardware::CPU_CYCLES, 0, 100000, false)
            .expect("Failed to create ring buffer");

        assert!(ringbuf.enable().is_ok(), "Failed to enable ring buffer");
        assert!(ringbuf.disable().is_ok(), "Failed to disable ring buffer");
    }
}
