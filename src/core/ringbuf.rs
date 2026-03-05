//! Ring buffer management for perf event sampling.
//!
//! This module provides a wrapper around `perf_event::Sampler` for managing
//! the memory-mapped ring buffer used to receive sample events from the kernel.
//!
//! # Example
//!
//! ```no_run
//! use perf_rs::core::ringbuf::{RingBuffer, RingBufferConfig};
//! use perf_event::events::Hardware;
//! use perf_event::Builder;
//!
//! // Create a sampler with a ring buffer
//! let counter = Builder::new(Hardware::INSTRUCTIONS)
//!     .sample_period(1_000_000)
//!     .build()?;
//!
//! let config = RingBufferConfig::default();
//! let mut ringbuf = RingBuffer::new(counter, config)?;
//!
//! // Enable and read samples
//! ringbuf.enable()?;
//! while let Some(record) = ringbuf.next_record() {
//!     // Process record...
//! }
//!
//! println!("Lost samples: {}", ringbuf.lost_count());
//! # Ok::<(), perf_rs::error::PerfError>(())
//! ```

use crate::error::{PerfError, Result};
use perf_event::events::Event;
use perf_event::{Builder, Counter, Record, Sampler};
use std::time::Duration;

/// Configuration options for ring buffer.
#[derive(Debug, Clone)]
pub struct RingBufferConfig {
    /// Size of the ring buffer in pages (will be rounded to power of 2).
    /// The actual buffer size is map_len * page_size.
    /// Minimum is 2 pages (one for control, one for data).
    pub map_len: usize,

    /// Whether to include lost sample tracking.
    pub track_lost: bool,
}

impl Default for RingBufferConfig {
    fn default() -> Self {
        Self {
            // Default to 16 pages (~64KB on most systems)
            // This is conservative to fit within perf_event_mlock_kb limits
            // (typically 512KB on default kernel configs)
            // Users can increase this if they have higher limits
            map_len: 16,
            track_lost: true,
        }
    }
}

impl RingBufferConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the ring buffer size in pages.
    ///
    /// The actual size will be rounded up to a power of two multiple
    /// of the system page size. At least 2 pages are required.
    pub fn with_map_len(mut self, map_len: usize) -> Self {
        self.map_len = map_len.max(2);
        self
    }

    /// Enable or disable lost sample tracking.
    pub fn with_track_lost(mut self, track_lost: bool) -> Self {
        self.track_lost = track_lost;
        self
    }
}

/// Statistics for ring buffer operations.
#[derive(Debug, Clone, Default)]
pub struct RingBufferStats {
    /// Number of samples lost due to buffer overflow.
    pub lost_samples: u64,

    /// Number of records successfully read.
    pub records_read: u64,

    /// Number of times the buffer wrapped around.
    pub wrap_count: u64,
}

/// A wrapper around `perf_event::Sampler` providing ring buffer management.
///
/// This type manages the memory-mapped ring buffer used to receive
/// sample events from the kernel. It tracks statistics about lost samples
/// and provides methods to read records from the buffer.
pub struct RingBuffer {
    /// The underlying sampler from perf-event2.
    sampler: Sampler,

    /// Configuration used to create this ring buffer.
    config: RingBufferConfig,

    /// Statistics for this ring buffer.
    stats: RingBufferStats,
}

impl RingBuffer {
    /// Create a new ring buffer from an existing counter.
    ///
    /// The counter must have been configured with sampling enabled
    /// (e.g., via `sample_period` or `sample_frequency`).
    ///
    /// # Arguments
    ///
    /// * `counter` - The counter to convert to a sampler
    /// * `config` - Configuration for the ring buffer
    ///
    /// # Returns
    ///
    /// Returns a `RingBuffer` on success, or a `PerfError` on failure.
    pub fn new(counter: Counter, config: RingBufferConfig) -> Result<Self> {
        let sampler = counter
            .sampled(config.map_len)
            .map_err(|e| PerfError::RingBufferSetup {
                message: format!("Failed to create sampler with map_len={}", config.map_len),
                source: Box::new(e),
            })?;

        Ok(Self {
            sampler,
            config,
            stats: RingBufferStats::default(),
        })
    }

    /// Create a new ring buffer from an event with default configuration.
    ///
    /// This is a convenience method that creates a counter and sampler
    /// in one step.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to sample
    /// * `sample_period` - The period at which to generate samples
    ///
    /// # Returns
    ///
    /// Returns a `RingBuffer` on success, or a `PerfError` on failure.
    pub fn from_event<E: Event + Clone + 'static>(event: E, sample_period: u64) -> Result<Self> {
        Self::from_event_with_config(event, sample_period, RingBufferConfig::default())
    }

    /// Create a new ring buffer to observe a specific process.
    ///
    /// This is a convenience method for profiling a specific PID.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to sample
    /// * `pid` - The process ID to observe
    /// * `sample_period` - The period at which to generate samples
    /// * `inherit` - Whether child processes should inherit the counters
    ///
    /// # Returns
    ///
    /// Returns a `RingBuffer` on success, or a `PerfError` on failure.
    pub fn from_event_for_pid<E: Event + Clone + 'static>(
        event: E,
        pid: i32,
        sample_period: u64,
        inherit: bool,
    ) -> Result<Self> {
        Self::from_event_for_pid_with_config(
            event,
            pid,
            sample_period,
            inherit,
            RingBufferConfig::default(),
        )
    }

    /// Create a new ring buffer to observe a specific process with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to sample
    /// * `pid` - The process ID to observe
    /// * `sample_period` - The period at which to generate samples
    /// * `inherit` - Whether child processes should inherit the counters
    /// * `config` - Configuration for the ring buffer
    ///
    /// # Returns
    ///
    /// Returns a `RingBuffer` on success, or a `PerfError` on failure.
    pub fn from_event_for_pid_with_config<E: Event + Clone + 'static>(
        event: E,
        pid: i32,
        sample_period: u64,
        inherit: bool,
        config: RingBufferConfig,
    ) -> Result<Self> {
        let counter = Builder::new(event)
            .observe_pid(pid)
            .inherit(inherit)
            .sample_period(sample_period)
            .build()
            .map_err(|e| PerfError::CounterSetup {
                source: Box::new(e),
            })?;

        Self::new(counter, config)
    }

    /// Create a new ring buffer from an event with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to sample
    /// * `sample_period` - The period at which to generate samples
    /// * `config` - Configuration for the ring buffer
    ///
    /// # Returns
    ///
    /// Returns a `RingBuffer` on success, or a `PerfError` on failure.
    pub fn from_event_with_config<E: Event + Clone + 'static>(
        event: E,
        sample_period: u64,
        config: RingBufferConfig,
    ) -> Result<Self> {
        let counter = Builder::new(event)
            .sample_period(sample_period)
            .build()
            .map_err(|e| PerfError::CounterSetup {
                source: Box::new(e),
            })?;

        Self::new(counter, config)
    }

    /// Enable the underlying counter.
    ///
    /// This must be called before samples can be collected.
    pub fn enable(&mut self) -> Result<()> {
        self.sampler.enable().map_err(|e| PerfError::CounterEnable {
            event_name: "sampler".to_string(),
            source: Box::new(e),
        })
    }

    /// Disable the underlying counter.
    ///
    /// Call this to stop collecting samples.
    pub fn disable(&mut self) -> Result<()> {
        self.sampler
            .disable()
            .map_err(|e| PerfError::CounterDisable {
                event_name: "sampler".to_string(),
                source: Box::new(e),
            })
    }

    /// Read the next record from the ring buffer (non-blocking).
    ///
    /// Returns `None` if no records are available.
    /// For blocking behavior, use `next_blocking`.
    ///
    /// # Returns
    ///
    /// Returns `Some(Record)` if a record is available, `None` otherwise.
    pub fn next_record(&mut self) -> Option<Record<'_>> {
        let record = self.sampler.next_record();
        if record.is_some() {
            self.stats.records_read += 1;
        }
        record
    }

    /// Read the next record from the ring buffer (blocking).
    ///
    /// This method will block until a record is available or the timeout
    /// is reached.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Optional timeout duration. `None` means wait forever.
    ///
    /// # Returns
    ///
    /// Returns `Some(Record)` if a record is available, `None` on timeout
    /// or if the observed process has exited.
    ///
    /// # Note
    ///
    /// This only works on Linux 3.18 and above.
    pub fn next_blocking(&mut self, timeout: Option<Duration>) -> Option<Record<'_>> {
        let record = self.sampler.next_blocking(timeout);
        if record.is_some() {
            self.stats.records_read += 1;
        }
        record
    }

    /// Process a record and update statistics.
    ///
    /// Call this after processing a record to update statistics like
    /// lost samples count.
    ///
    /// # Arguments
    ///
    /// * `record` - The record to process
    pub fn process_record_stats(&mut self, record: &Record<'_>) {
        if record.data().len() > 1 {
            self.stats.wrap_count += 1;
        }

        if self.config.track_lost {
            self.check_for_lost_samples(record);
        }
    }

    /// Check a record for lost samples.
    ///
    /// PERF_RECORD_LOST records indicate that samples were dropped
    /// because the ring buffer was full.
    fn check_for_lost_samples(&mut self, record: &Record<'_>) {
        // Record type 2 is PERF_RECORD_LOST
        // See perf_event.h in the kernel source
        const PERF_RECORD_LOST: u32 = 2;

        if record.ty() == PERF_RECORD_LOST {
            // The record format for PERF_RECORD_LOST is:
            // struct {
            //     struct perf_event_header header;
            //     u64 lost;  // number of lost samples
            //     u64 id;    // id of the counter
            // }
            let data = record.to_vec();
            if data.len() >= 24 {
                // Skip header (8 bytes), read lost count
                let lost = u64::from_ne_bytes([
                    data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
                ]);
                self.stats.lost_samples += lost;
            }
        }
    }

    /// Get the number of samples lost due to buffer overflow.
    pub fn lost_count(&self) -> u64 {
        self.stats.lost_samples
    }

    /// Get the current statistics for this ring buffer.
    pub fn stats(&self) -> &RingBufferStats {
        &self.stats
    }

    /// Reset statistics counters.
    pub fn reset_stats(&mut self) {
        self.stats = RingBufferStats::default();
    }

    /// Get the configuration used for this ring buffer.
    pub fn config(&self) -> &RingBufferConfig {
        &self.config
    }

    /// Access the underlying sampler.
    ///
    /// This provides direct access to the `Sampler` for advanced use cases.
    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    /// Access the underlying sampler mutably.
    ///
    /// This provides mutable access to the `Sampler` for advanced use cases.
    pub fn sampler_mut(&mut self) -> &mut Sampler {
        &mut self.sampler
    }

    /// Convert this ring buffer back into a counter.
    ///
    /// This will close the ring buffer.
    pub fn into_counter(self) -> Counter {
        self.sampler.into_counter()
    }
}

/// Builder for creating a ring buffer with custom settings.
///
/// # Example
///
/// ```no_run
/// use perf_rs::core::ringbuf::RingBufferBuilder;
/// use perf_event::events::Hardware;
///
/// let mut ringbuf = RingBufferBuilder::new(Hardware::CPU_CYCLES)
///     .sample_period(100_000)
///     .map_len(256)
///     .build()?;
///
/// ringbuf.enable()?;
/// # Ok::<(), perf_rs::error::PerfError>(())
/// ```
pub struct RingBufferBuilder<E: Event + Clone + 'static> {
    event: E,
    sample_period: u64,
    sample_frequency: Option<u64>,
    config: RingBufferConfig,
}

impl<E: Event + Clone + 'static> RingBufferBuilder<E> {
    /// Create a new builder for the given event.
    pub fn new(event: E) -> Self {
        Self {
            event,
            sample_period: 1_000_000,
            sample_frequency: None,
            config: RingBufferConfig::default(),
        }
    }

    /// Set the sample period (number of events between samples).
    ///
    /// This is mutually exclusive with `sample_frequency`.
    pub fn sample_period(mut self, period: u64) -> Self {
        self.sample_period = period;
        self.sample_frequency = None;
        self
    }

    /// Set the sample frequency (samples per second).
    ///
    /// This is mutually exclusive with `sample_period`.
    pub fn sample_frequency(mut self, freq: u64) -> Self {
        self.sample_frequency = Some(freq);
        self
    }

    /// Set the ring buffer size in pages.
    pub fn map_len(mut self, map_len: usize) -> Self {
        self.config.map_len = map_len.max(2);
        self
    }

    /// Enable or disable lost sample tracking.
    pub fn track_lost(mut self, track: bool) -> Self {
        self.config.track_lost = track;
        self
    }

    /// Build the ring buffer.
    pub fn build(self) -> Result<RingBuffer> {
        let mut builder = Builder::new(self.event);

        if let Some(freq) = self.sample_frequency {
            builder.sample_frequency(freq);
        } else {
            builder.sample_period(self.sample_period);
        }

        let counter = builder.build().map_err(|e| PerfError::CounterSetup {
            source: Box::new(e),
        })?;

        RingBuffer::new(counter, self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perf_event::events::Hardware;

    #[test]
    fn test_ring_buffer_config_default() {
        let config = RingBufferConfig::default();
        assert_eq!(config.map_len, 16);
        assert!(config.track_lost);
    }

    #[test]
    fn test_ring_buffer_config_builder() {
        let config = RingBufferConfig::new()
            .with_map_len(256)
            .with_track_lost(false);

        assert_eq!(config.map_len, 256);
        assert!(!config.track_lost);
    }

    #[test]
    fn test_ring_buffer_config_min_pages() {
        // Should enforce minimum of 2 pages
        let config = RingBufferConfig::new().with_map_len(1);
        assert_eq!(config.map_len, 2);
    }

    #[test]
    fn test_ring_buffer_stats_default() {
        let stats = RingBufferStats::default();
        assert_eq!(stats.lost_samples, 0);
        assert_eq!(stats.records_read, 0);
        assert_eq!(stats.wrap_count, 0);
    }

    #[test]
    fn test_ring_buffer_creation() {
        // This test may fail if we don't have permissions
        let result = RingBuffer::from_event(Hardware::INSTRUCTIONS, 1_000_000);

        match result {
            Ok(mut ringbuf) => {
                // Verify initial state
                assert_eq!(ringbuf.lost_count(), 0);
                assert_eq!(ringbuf.stats().records_read, 0);

                // Try to enable
                match ringbuf.enable() {
                    Ok(_) => println!("Ring buffer enabled successfully"),
                    Err(e) => println!("Enable failed (expected without permissions): {}", e),
                }

                // Try to disable
                match ringbuf.disable() {
                    Ok(_) => println!("Ring buffer disabled successfully"),
                    Err(e) => println!("Disable failed: {}", e),
                }
            }
            Err(e) => {
                println!(
                    "Ring buffer creation failed (expected without permissions): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_ring_buffer_builder() {
        let result = RingBufferBuilder::new(Hardware::CPU_CYCLES)
            .sample_period(500_000)
            .map_len(64)
            .track_lost(true)
            .build();

        match result {
            Ok(ringbuf) => {
                assert_eq!(ringbuf.config().map_len, 64);
                assert!(ringbuf.config().track_lost);
            }
            Err(e) => {
                println!(
                    "Ring buffer creation failed (expected without permissions): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_ring_buffer_record_reading() {
        let result = RingBuffer::from_event(Hardware::INSTRUCTIONS, 1_000_000);

        match result {
            Ok(mut ringbuf) => {
                // Enable the counter
                if ringbuf.enable().is_ok() {
                    // Do some work to generate events
                    let _v: Vec<u64> = (0..1000).collect();

                    // Try to read a record (non-blocking)
                    // May or may not get a record depending on timing
                    let _ = ringbuf.next_record();

                    // Disable the counter
                    let _ = ringbuf.disable();
                }
            }
            Err(e) => {
                println!(
                    "Ring buffer creation failed (expected without permissions): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_ring_buffer_stats_tracking() {
        let config = RingBufferConfig::new()
            .with_map_len(16)
            .with_track_lost(true);

        let result = RingBuffer::from_event_with_config(Hardware::CPU_CYCLES, 100_000, config);

        match result {
            Ok(mut ringbuf) => {
                // Initial stats should be zero
                assert_eq!(ringbuf.stats().records_read, 0);

                // Reset stats
                ringbuf.reset_stats();
                assert_eq!(ringbuf.stats().records_read, 0);
            }
            Err(e) => {
                println!(
                    "Ring buffer creation failed (expected without permissions): {}",
                    e
                );
            }
        }
    }
}
