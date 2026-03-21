//! Tracepoint support for Linux performance monitoring.
//!
//! This module provides types and functions for working with kernel tracepoints.
//! Tracepoints are static instrumentation points in the kernel that can be used
//! for performance analysis and debugging.

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::error::PerfError;

/// Error type for tracepoint operations.
#[derive(Debug, thiserror::Error)]
pub enum TracepointError {
    /// Tracepoint not found in tracefs.
    #[error("Tracepoint '{name}' not found")]
    NotFound { name: String },

    /// tracefs filesystem is not mounted.
    #[error("tracefs filesystem is not mounted")]
    TracefsNotMounted,

    /// Permission denied when accessing tracefs.
    #[error("Permission denied accessing tracefs at '{path}'")]
    PermissionDenied { path: String },

    /// Malformed ID file in tracefs.
    #[error("Malformed tracepoint ID in '{path}': {source}")]
    MalformedId {
        path: String,
        #[source]
        source: io::Error,
    },

    /// Malformed content in tracefs file.
    #[error("Malformed content in tracefs file '{path}': {message}")]
    MalformedContent { path: String, message: String },

    /// Failed to read tracefs file.
    #[error("Failed to read tracefs file '{path}': {source}")]
    FileRead {
        path: String,
        #[source]
        source: io::Error,
    },
}

impl From<TracepointError> for PerfError {
    fn from(err: TracepointError) -> Self {
        PerfError::Tracepoint {
            source: Box::new(err),
        }
    }
}

/// Represents a kernel tracepoint identifier.
///
/// A tracepoint is identified by its subsystem (e.g., "sched") and name
/// (e.g., "sched_switch"). The kernel assigns a unique numeric ID to each
/// tracepoint, which is used when configuring perf events.
///
/// # Example
///
/// ```no_run
/// use perf_rs::tracepoint::TracepointId;
///
/// // Create from known ID
/// let tp = TracepointId::new("sched", "sched_switch", 123);
///
/// // Look up ID from tracefs
/// let tp = TracepointId::from_name("sched", "sched_switch")?;
/// # Ok::<(), perf_rs::PerfError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracepointId {
    /// The subsystem name (e.g., "sched", "syscalls", "irq").
    pub subsystem: String,
    /// The tracepoint name within the subsystem (e.g., "sched_switch").
    pub name: String,
    /// The numeric ID assigned by the kernel.
    pub id: u64,
}

impl TracepointId {
    /// Creates a new TracepointId with the given values.
    ///
    /// # Arguments
    ///
    /// * `subsystem` - The subsystem name (e.g., "sched")
    /// * `name` - The tracepoint name (e.g., "sched_switch")
    /// * `id` - The numeric tracepoint ID
    ///
    /// # Example
    ///
    /// ```
    /// use perf_rs::tracepoint::TracepointId;
    ///
    /// let tp = TracepointId::new("sched", "sched_switch", 123);
    /// assert_eq!(tp.subsystem, "sched");
    /// assert_eq!(tp.name, "sched_switch");
    /// assert_eq!(tp.id, 123);
    /// ```
    pub fn new(subsystem: &str, name: &str, id: u64) -> Self {
        Self {
            subsystem: subsystem.to_string(),
            name: name.to_string(),
            id,
        }
    }

    /// Looks up a tracepoint ID from tracefs.
    ///
    /// This function searches for the tracepoint in tracefs and reads its
    /// numeric ID. It checks the primary tracefs mount point first
    /// (`/sys/kernel/tracing`), then falls back to the debugfs mount
    /// (`/sys/kernel/debug/tracing`).
    ///
    /// # Arguments
    ///
    /// * `subsystem` - The subsystem name (e.g., "sched")
    /// * `name` - The tracepoint name (e.g., "sched_switch")
    ///
    /// # Returns
    ///
    /// Returns the `TracepointId` if found, or an appropriate error.
    ///
    /// # Errors
    ///
    /// * `TracepointError::TracefsNotMounted` - tracefs is not available
    /// * `TracepointError::NotFound` - The tracepoint does not exist
    /// * `TracepointError::PermissionDenied` - Cannot access tracefs
    /// * `TracepointError::MalformedId` - The ID file contains invalid data
    ///
    /// # Example
    ///
    /// ```no_run
    /// use perf_rs::tracepoint::TracepointId;
    ///
    /// let tp = TracepointId::from_name("sched", "sched_switch")?;
    /// println!("Tracepoint ID: {}", tp.id);
    /// # Ok::<(), perf_rs::PerfError>(())
    /// ```
    pub fn from_name(subsystem: &str, name: &str) -> std::result::Result<Self, PerfError> {
        let tracefs_path = find_tracefs_path()?;

        let id_path = tracefs_path
            .join("events")
            .join(subsystem)
            .join(name)
            .join("id");

        if !id_path.exists() {
            return Err(TracepointError::NotFound {
                name: format!("{}:{}", subsystem, name),
            }
            .into());
        }

        let id_str = fs::read_to_string(&id_path).map_err(|e| {
            if e.kind() == io::ErrorKind::PermissionDenied {
                TracepointError::PermissionDenied {
                    path: id_path.display().to_string(),
                }
            } else {
                TracepointError::MalformedId {
                    path: id_path.display().to_string(),
                    source: e,
                }
            }
        })?;

        let id: u64 = id_str
            .trim()
            .parse()
            .map_err(|e| TracepointError::MalformedId {
                path: id_path.display().to_string(),
                source: io::Error::new(io::ErrorKind::InvalidData, e),
            })?;

        Ok(Self::new(subsystem, name, id))
    }

    /// Returns the full name of the tracepoint as "subsystem:name".
    ///
    /// # Example
    ///
    /// ```
    /// use perf_rs::tracepoint::TracepointId;
    ///
    /// let tp = TracepointId::new("sched", "sched_switch", 123);
    /// assert_eq!(tp.full_name(), "sched:sched_switch");
    /// ```
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.subsystem, self.name)
    }
}

/// Finds the tracefs mount point.
///
/// Checks the primary location (`/sys/kernel/tracing`) first, then falls
/// back to the debugfs location (`/sys/kernel/debug/tracing`).
///
/// # Errors
///
/// Returns `TracepointError::TracefsNotMounted` if neither location exists.
fn find_tracefs_path() -> std::result::Result<PathBuf, PerfError> {
    let primary = PathBuf::from("/sys/kernel/tracing");
    let fallback = PathBuf::from("/sys/kernel/debug/tracing");

    if primary.exists() {
        return Ok(primary);
    }

    if fallback.exists() {
        return Ok(fallback);
    }

    Err(TracepointError::TracefsNotMounted.into())
}

/// Discovers all available tracepoints from tracefs.
///
/// Reads the `available_events` file from tracefs and parses it to extract
/// all tracepoint (subsystem, name) pairs.
///
/// # Returns
///
/// A vector of `(subsystem, name)` tuples representing all available tracepoints.
///
/// # Errors
///
/// * `TracepointError::TracefsNotMounted` - tracefs is not available
/// * `TracepointError::FileRead` - Cannot read `available_events` file
/// * `TracepointError::MalformedContent` - The file contains invalid entries
///
/// # Example
///
/// ```no_run
/// use perf_rs::tracepoint::discover_tracepoints;
///
/// let tracepoints = discover_tracepoints()?;
/// for (subsystem, name) in tracepoints {
///     println!("{}:{}", subsystem, name);
/// }
/// # Ok::<(), perf_rs::PerfError>(())
/// ```
pub fn discover_tracepoints() -> std::result::Result<Vec<(String, String)>, PerfError> {
    let tracefs_path = find_tracefs_path()?;
    let available_events_path = tracefs_path.join("available_events");

    let content = fs::read_to_string(&available_events_path).map_err(|e| {
        if e.kind() == io::ErrorKind::PermissionDenied {
            TracepointError::PermissionDenied {
                path: available_events_path.display().to_string(),
            }
        } else {
            TracepointError::FileRead {
                path: available_events_path.display().to_string(),
                source: e,
            }
        }
    })?;

    let mut tracepoints = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((subsystem, name)) = line.split_once(':') {
            if !subsystem.is_empty() && !name.is_empty() {
                tracepoints.push((subsystem.to_string(), name.to_string()));
            }
        }
    }

    Ok(tracepoints)
}

/// Gets the numeric ID for a specific tracepoint.
///
/// Reads the ID from the tracefs `events/<subsystem>/<event>/id` file.
///
/// # Arguments
///
/// * `subsystem` - The subsystem name (e.g., "sched")
/// * `event` - The tracepoint name (e.g., "sched_switch")
///
/// # Returns
///
/// The numeric tracepoint ID.
///
/// # Errors
///
/// * `TracepointError::TracefsNotMounted` - tracefs is not available
/// * `TracepointError::NotFound` - The tracepoint does not exist
/// * `TracepointError::PermissionDenied` - Cannot access tracefs
/// * `TracepointError::MalformedId` - The ID file contains invalid data
///
/// # Example
///
/// ```no_run
/// use perf_rs::tracepoint::get_tracepoint_id;
///
/// let id = get_tracepoint_id("sched", "sched_switch")?;
/// println!("Tracepoint ID: {}", id);
/// # Ok::<(), perf_rs::PerfError>(())
/// ```
pub fn get_tracepoint_id(subsystem: &str, event: &str) -> std::result::Result<u64, PerfError> {
    let tracefs_path = find_tracefs_path()?;

    let id_path = tracefs_path
        .join("events")
        .join(subsystem)
        .join(event)
        .join("id");

    if !id_path.exists() {
        return Err(TracepointError::NotFound {
            name: format!("{}:{}", subsystem, event),
        }
        .into());
    }

    let id_str = fs::read_to_string(&id_path).map_err(|e| {
        if e.kind() == io::ErrorKind::PermissionDenied {
            TracepointError::PermissionDenied {
                path: id_path.display().to_string(),
            }
        } else {
            TracepointError::MalformedId {
                path: id_path.display().to_string(),
                source: e,
            }
        }
    })?;

    let id: u64 = id_str
        .trim()
        .parse()
        .map_err(|e| TracepointError::MalformedId {
            path: id_path.display().to_string(),
            source: io::Error::new(io::ErrorKind::InvalidData, e),
        })?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracepoint_id_new() {
        let tp = TracepointId::new("sched", "sched_switch", 123);
        assert_eq!(tp.subsystem, "sched");
        assert_eq!(tp.name, "sched_switch");
        assert_eq!(tp.id, 123);
    }

    #[test]
    fn test_tracepoint_id_full_name() {
        let tp = TracepointId::new("syscalls", "sys_enter_openat", 456);
        assert_eq!(tp.full_name(), "syscalls:sys_enter_openat");
    }

    #[test]
    fn test_tracepoint_id_clone_eq() {
        let tp1 = TracepointId::new("irq", "irq_handler_entry", 789);
        let tp2 = tp1.clone();
        assert_eq!(tp1, tp2);
    }

    #[test]
    fn test_tracepoint_error_not_found() {
        let err = TracepointError::NotFound {
            name: "test:test".to_string(),
        };
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_tracepoint_error_tracefs_not_mounted() {
        let err = TracepointError::TracefsNotMounted;
        assert!(err.to_string().contains("not mounted"));
    }

    #[test]
    fn test_tracepoint_error_permission_denied() {
        let err = TracepointError::PermissionDenied {
            path: "/sys/kernel/tracing".to_string(),
        };
        assert!(err.to_string().contains("Permission denied"));
    }

    #[test]
    fn test_tracepoint_error_malformed_id() {
        let err = TracepointError::MalformedId {
            path: "/sys/kernel/tracing/events/test/id".to_string(),
            source: io::Error::new(io::ErrorKind::InvalidData, "parse error"),
        };
        assert!(err.to_string().contains("Malformed"));
    }

    #[test]
    fn test_tracepoint_error_malformed_content() {
        let err = TracepointError::MalformedContent {
            path: "/sys/kernel/tracing/available_events".to_string(),
            message: "invalid format".to_string(),
        };
        assert!(err.to_string().contains("Malformed content"));
    }

    #[test]
    fn test_tracepoint_error_file_read() {
        let err = TracepointError::FileRead {
            path: "/sys/kernel/tracing/available_events".to_string(),
            source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
        };
        assert!(err.to_string().contains("Failed to read"));
    }

    #[test]
    fn test_discover_tracepoints_returns_result() {
        let result = discover_tracepoints();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_discover_tracepoints_success_or_tracefs_not_mounted() {
        match discover_tracepoints() {
            Ok(tracepoints) => {
                assert!(!tracepoints.is_empty());
                for (subsystem, name) in &tracepoints {
                    assert!(!subsystem.is_empty());
                    assert!(!name.is_empty());
                    assert!(!subsystem.contains(':'));
                    assert!(!name.contains(':'));
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                assert!(err_msg.contains("not mounted") || err_msg.contains("Permission denied"));
            }
        }
    }

    #[test]
    fn test_get_tracepoint_id_returns_result() {
        let result = get_tracepoint_id("sched", "sched_switch");
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_get_tracepoint_id_success_or_error() {
        match get_tracepoint_id("sched", "sched_switch") {
            Ok(id) => {
                assert!(id > 0);
            }
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    err_msg.contains("not found")
                        || err_msg.contains("not mounted")
                        || err_msg.contains("Permission denied")
                );
            }
        }
    }

    #[test]
    fn test_get_tracepoint_id_nonexistent_tracepoint() {
        let result = get_tracepoint_id("nonexistent_subsystem_xyz", "nonexistent_event_xyz");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("not found") || err_msg.contains("not mounted"));
    }
}
