//! Error types for the perf-rs library.
//!
//! This module defines comprehensive error types for all operations
//! in the perf-rs performance monitoring library.

use std::path::PathBuf;
use thiserror::Error;

/// The main error type for perf-rs operations.
#[derive(Debug, Error)]
pub enum PerfError {
    /// Failed to attach to a process.
    #[error("Failed to attach to process {pid}: {source}")]
    ProcessAttach {
        /// Process ID that failed to attach
        pid: u32,
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Process not found.
    #[error("Process {pid} not found: {source}")]
    ProcessNotFound {
        /// Process ID that was not found
        pid: u32,
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to set up performance counter.
    #[error("Failed to set up performance counter: {source}")]
    CounterSetup {
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to enable performance counter.
    #[error("Failed to enable performance counter '{event_name}': {source}")]
    CounterEnable {
        /// Name of the event that failed to enable
        event_name: String,
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to disable performance counter.
    #[error("Failed to disable performance counter '{event_name}': {source}")]
    CounterDisable {
        /// Name of the event that failed to disable
        event_name: String,
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to read counter value.
    #[error("Failed to read counter value for '{event_name}': {source}")]
    CounterRead {
        /// Name of the event that failed to read
        event_name: String,
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Permission denied for operation.
    #[error("Permission denied for operation: {operation}")]
    PermissionDenied {
        /// Description of the operation that was denied
        operation: String,
    },

    /// Invalid configuration.
    #[error("Invalid configuration: {message}")]
    InvalidConfig {
        /// Description of the configuration error
        message: String,
    },

    /// File not found.
    #[error("File not found: {path}")]
    FileNotFound {
        /// Path to the file that was not found
        path: PathBuf,
    },

    /// Failed to parse file.
    #[error("Failed to parse file '{path}': {source}")]
    FileParse {
        /// Path to the file that failed to parse
        path: PathBuf,
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to read file.
    #[error("Failed to read file '{path}': {source}")]
    FileRead {
        /// Path to the file that failed to read
        path: PathBuf,
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to write file.
    #[error("Failed to write file '{path}': {source}")]
    FileWrite {
        /// Path to the file that failed to write
        path: PathBuf,
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Invalid event name or configuration.
    #[error("Invalid event: {event_name} - {reason}")]
    InvalidEvent {
        /// Name of the invalid event
        event_name: String,
        /// Reason why the event is invalid
        reason: String,
    },

    /// System call failed.
    #[error("System call '{syscall}' failed: {source}")]
    SyscallError {
        /// Name of the system call that failed
        syscall: String,
        /// Underlying error source
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error with message.
    #[error("{0}")]
    Other(String),
}

/// Type alias for Result with PerfError.
pub type Result<T> = std::result::Result<T, PerfError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PerfError::ProcessNotFound {
            pid: 1234,
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no such process",
            )),
        };
        assert!(err.to_string().contains("1234"));
    }

    #[test]
    fn test_permission_denied() {
        let err = PerfError::PermissionDenied {
            operation: "access perf events".to_string(),
        };
        assert!(err.to_string().contains("Permission denied"));
        assert!(err.to_string().contains("access perf events"));
    }

    #[test]
    fn test_invalid_config() {
        let err = PerfError::InvalidConfig {
            message: "sampling period must be positive".to_string(),
        };
        assert!(err.to_string().contains("Invalid configuration"));
    }

    #[test]
    fn test_file_not_found() {
        let err = PerfError::FileNotFound {
            path: PathBuf::from("/proc/1234/status"),
        };
        assert!(err.to_string().contains("/proc/1234/status"));
    }
}
