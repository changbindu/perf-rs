use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during DWARF-based stack unwinding.
#[derive(Debug, Error)]
pub enum UnwindError {
    #[error("No unwind information found for address 0x{address:x}")]
    NoEhFrame { address: u64 },

    #[error("Invalid CFI data: {message}")]
    InvalidCfi { message: String },

    #[error("Failed to read stack at address 0x{address:x}")]
    StackReadFailed { address: u64 },

    #[error("Stack unwinding exceeded maximum depth of {depth}")]
    MaxDepthExceeded { depth: usize },

    #[error("Binary file not found: {path}")]
    BinaryNotFound { path: PathBuf },

    #[error("Register value not available: {register}")]
    RegisterNotFound { register: u16 },
}

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

    #[error("System-wide profiling permission denied. Requires perf_event_paranoid <= 0 or CAP_PERFMON/CAP_SYS_ADMIN capability")]
    SystemWidePermissionDenied,

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

    #[error("Failed to attach to process {pid}: {source}")]
    ProcessAttach {
        pid: u32,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to fork process: {source}")]
    ProcessFork {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to execute command '{command}': {source}")]
    CommandExecution {
        command: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to send signal to process {pid}: {source}")]
    SignalSend {
        pid: i32,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to wait for process: {source}")]
    ProcessWait {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },

    #[error("Invalid CPU list: {message}")]
    InvalidCpuList { message: String },

    #[error("CPU {cpu_id} out of range (max: {max_cpu})")]
    CpuOutOfRange { cpu_id: u32, max_cpu: u32 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tracepoint error: {source}")]
    Tracepoint {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Stack unwinding failed: {0}")]
    Unwind(#[from] UnwindError),
}

pub type Result<T> = std::result::Result<T, PerfError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwind_error_display() {
        assert_eq!(
            UnwindError::NoEhFrame { address: 0x1234 }.to_string(),
            "No unwind information found for address 0x1234"
        );

        assert_eq!(
            UnwindError::InvalidCfi {
                message: "bad CFI".to_string()
            }
            .to_string(),
            "Invalid CFI data: bad CFI"
        );

        assert_eq!(
            UnwindError::StackReadFailed { address: 0xabcd }.to_string(),
            "Failed to read stack at address 0xabcd"
        );

        assert_eq!(
            UnwindError::MaxDepthExceeded { depth: 100 }.to_string(),
            "Stack unwinding exceeded maximum depth of 100"
        );

        assert_eq!(
            UnwindError::BinaryNotFound {
                path: PathBuf::from("/usr/bin/test")
            }
            .to_string(),
            "Binary file not found: /usr/bin/test"
        );

        assert_eq!(
            UnwindError::RegisterNotFound { register: 7 }.to_string(),
            "Register value not available: 7"
        );
    }

    #[test]
    fn test_unwind_error_into_perf_error() {
        let unwind_err = UnwindError::MaxDepthExceeded { depth: 50 };
        let perf_err: PerfError = unwind_err.into();
        assert!(matches!(perf_err, PerfError::Unwind(_)));
        assert!(perf_err.to_string().contains("maximum depth"));
    }
}
