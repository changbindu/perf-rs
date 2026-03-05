use crate::error::{PerfError, Result};
use perf_event::events::Event;
use perf_event::{Builder, Counter, Record, Sampler};

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
