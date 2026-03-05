use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PerfError {
    #[error("Failed to set up performance counter: {source}")]
    CounterSetup {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to enable performance counter '{event_name}': {source}")]
    CounterEnable {
        event_name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to disable performance counter '{event_name}': {source}")]
    CounterDisable {
        event_name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to read counter value for '{event_name}': {source}")]
    CounterRead {
        event_name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Permission denied for operation: {operation}")]
    PermissionDenied { operation: String },

    #[error("Failed to read file '{path}': {source}")]
    FileRead {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to parse ELF file '{path}': {source}")]
    ElfParse {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to read kernel symbols: {source}")]
    KernelSymbols {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to set up ring buffer: {message}")]
    RingBufferSetup {
        message: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Invalid perf.data magic: expected '{expected}', got '{actual}'")]
    InvalidMagic { expected: String, actual: String },

    #[error("Unsupported perf.data version: {version}")]
    UnsupportedVersion { version: u32 },

    #[error("Invalid event type in perf.data: {event_type}")]
    InvalidEventType { event_type: u16 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, PerfError>;
