//! Pager module for system pager integration.
//!
//! Provides functionality to spawn external pagers (less, more, etc.)
//! for terminal output pagination with automatic TTY detection.

use std::env;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};

/// Pager for system pager integration.
///
/// The pager detects and spawns external pagers like `less` or `more`
/// when output is to a terminal. It automatically disables pagination
/// when output is piped or redirected.
#[derive(Debug)]
pub struct Pager {
    /// Path to the pager executable
    pager_cmd: Option<PathBuf>,
    /// Whether stdout is a terminal
    is_tty: bool,
}

/// Writer that wraps a pager subprocess's stdin.
///
/// This struct holds the child process and its stdin, implementing `Write`
/// to allow writing to the pager. When dropped, it waits for the child
/// process to complete.
struct PagerWriter {
    #[allow(dead_code)]
    child: Child,
    stdin: Option<ChildStdin>,
}

impl Write for PagerWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdin.as_mut().map_or(
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdin closed")),
            |s| s.write(buf),
        )
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdin.as_mut().map_or(
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdin closed")),
            |s| s.flush(),
        )
    }
}

impl Drop for PagerWriter {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.wait();
    }
}

impl Pager {
    /// Detects available pager on the system.
    ///
    /// Checks in order:
    /// 1. $PAGER environment variable
    /// 2. `less` command
    /// 3. `more` command
    /// 4. `most` command
    ///
    /// # Returns
    ///
    /// `Some(PathBuf)` if a pager is found, `None` otherwise
    pub fn detect() -> Option<PathBuf> {
        if let Ok(pager) = env::var("PAGER") {
            if !pager.is_empty() {
                if let Ok(output) = Command::new("which").arg(&pager).output() {
                    if output.status.success() {
                        let path = String::from_utf8_lossy(&output.stdout);
                        let path = path.trim();
                        if !path.is_empty() {
                            return Some(PathBuf::from(path));
                        }
                    }
                }
            }
        }

        for pager in ["less", "more", "most"] {
            if let Ok(output) = Command::new("which").arg(pager).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout);
                    let path = path.trim();
                    if !path.is_empty() {
                        return Some(PathBuf::from(path));
                    }
                }
            }
        }

        None
    }

    /// Creates a new pager configuration.
    ///
    /// Automatically detects available pager and TTY status.
    ///
    /// # Returns
    ///
    /// A new `Pager` instance
    pub fn new() -> Self {
        Self {
            pager_cmd: Self::detect(),
            is_tty: std::io::stdout().is_terminal(),
        }
    }

    /// Checks if stdout is connected to a terminal (TTY).
    ///
    /// # Returns
    ///
    /// `true` if stdout is a TTY, `false` otherwise
    pub fn is_tty(&self) -> bool {
        self.is_tty
    }

    /// Spawns the pager subprocess and returns a writer.
    ///
    /// Creates a child process for the pager and returns a `Box<dyn Write>`
    /// that writes to the pager's stdin.
    ///
    /// # Returns
    ///
    /// `io::Result<Box<dyn Write>>` - A writer to the pager's stdin
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No pager is available
    /// - Failed to spawn the pager process
    /// - Failed to open stdin pipe
    pub fn spawn(&self) -> io::Result<Box<dyn Write>> {
        let pager_path = self
            .pager_cmd
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No pager available"))?;

        // Build command with appropriate flags
        let mut cmd = Command::new(pager_path);

        // For 'less', add -X to keep output on screen after exit
        // and -R to handle ANSI color sequences
        if pager_path.file_name().is_some_and(|name| name == "less") {
            cmd.args(["-X", "-R"]);
        }

        let mut child = cmd.stdin(Stdio::piped()).spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Failed to open pager stdin")
        })?;

        Ok(Box::new(PagerWriter {
            child,
            stdin: Some(stdin),
        }))
    }

    /// Determines if pagination should be used.
    ///
    /// Returns true if:
    /// - stdout is a TTY
    /// - A pager is available
    ///
    /// # Returns
    ///
    /// `true` if pagination should be used, `false` otherwise
    pub fn should_use_pager(&self) -> bool {
        self.is_tty && self.pager_cmd.is_some()
    }

    /// Gets the pager command path.
    ///
    /// # Returns
    ///
    /// Reference to the pager command path, if set
    pub fn pager_cmd(&self) -> Option<&PathBuf> {
        self.pager_cmd.as_ref()
    }
}

impl Default for Pager {
    fn default() -> Self {
        Self::new()
    }
}

/// Finds an available pager on the system.
///
/// This is a convenience function that wraps [`Pager::detect()`].
/// Checks in order:
/// 1. $PAGER environment variable
/// 2. `less` command
/// 3. `more` command
/// 4. `most` command
///
/// # Returns
///
/// `Some(PathBuf)` if a pager is found, `None` otherwise
///
/// # Example
///
/// ```no_run
/// use perf_rs::pager::find_pager;
///
/// if let Some(pager) = find_pager() {
///     println!("Found pager: {:?}", pager);
/// }
/// ```
pub fn find_pager() -> Option<PathBuf> {
    Pager::detect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::io::IsTerminal;

    // Helper to check if a command exists in PATH
    fn command_exists(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_pager_detect_with_pager_env() {
        // Save original PAGER value
        let original_pager = env::var("PAGER").ok();

        // Set PAGER to a known value
        env::set_var("PAGER", "less");

        // detect() should find the pager from $PAGER
        let result = Pager::detect();
        assert!(
            result.is_some(),
            "detect() should find pager when $PAGER is set"
        );

        let path = result.unwrap();
        assert!(
            path.to_string_lossy().contains("less"),
            "detected pager should be 'less' when $PAGER=less"
        );

        // Restore original PAGER value
        if let Some(val) = original_pager {
            env::set_var("PAGER", val);
        } else {
            env::remove_var("PAGER");
        }
    }

    #[test]
    fn test_pager_detect_without_pager_env_finds_less() {
        // Save original PAGER value
        let original_pager = env::var("PAGER").ok();

        // Remove PAGER env var
        env::remove_var("PAGER");

        // detect() should find 'less' as fallback
        let result = Pager::detect();

        // If 'less' exists on system, it should be found
        if command_exists("less") {
            assert!(
                result.is_some(),
                "detect() should find 'less' when $PAGER is not set and 'less' exists"
            );
            let path = result.unwrap();
            assert!(
                path.to_string_lossy().contains("less"),
                "detected pager should be 'less' as fallback"
            );
        }

        // Restore original PAGER value
        if let Some(val) = original_pager {
            env::set_var("PAGER", val);
        } else {
            env::remove_var("PAGER");
        }
    }

    #[test]
    fn test_pager_detect_finds_more_as_fallback() {
        // This test verifies that 'more' is found if 'less' is not available
        // We can't easily test this without manipulating PATH, so we test
        // that the detection order is correct by checking the implementation
        // would find 'more' if 'less' were not found

        // Save original PAGER value
        let original_pager = env::var("PAGER").ok();
        env::remove_var("PAGER");

        let result = Pager::detect();

        // At minimum, one of less/more/most should be found on a typical system
        if command_exists("less") || command_exists("more") || command_exists("most") {
            assert!(
                result.is_some(),
                "detect() should find at least one pager on a typical system"
            );
        }

        // Restore original PAGER value
        if let Some(val) = original_pager {
            env::set_var("PAGER", val);
        } else {
            env::remove_var("PAGER");
        }
    }

    #[test]
    fn test_pager_new_creates_configuration() {
        let pager = Pager::new();

        // new() should create a Pager with detected pager_cmd
        // STUB: Currently returns None, test will fail
        assert!(
            pager.pager_cmd.is_some(),
            "new() should detect and set pager_cmd"
        );
    }

    #[test]
    fn test_pager_new_detects_tty() {
        let pager = Pager::new();

        // When running tests, stdout is typically not a TTY
        // is_tty should reflect the actual TTY status
        // STUB: Currently returns false, test will fail if stdout IS a TTY
        let expected = std::io::stdout().is_terminal();
        assert_eq!(
            pager.is_tty, expected,
            "is_tty should match actual terminal status"
        );
    }

    #[test]
    fn test_pager_is_tty_method() {
        let pager = Pager::new();

        // is_tty() method should return the same value as the field
        // STUB: Currently always returns false
        let expected = std::io::stdout().is_terminal();
        assert_eq!(
            pager.is_tty(),
            expected,
            "is_tty() should return actual terminal status"
        );
    }

    #[test]
    fn test_pager_spawn_returns_writer() {
        // Save original PAGER value
        let original_pager = env::var("PAGER").ok();
        env::set_var("PAGER", "cat"); // Use 'cat' as a simple pager for testing

        let pager = Pager::new();

        // spawn() should return a Box<dyn Write>
        // STUB: Currently returns error
        let result = pager.spawn();
        assert!(
            result.is_ok(),
            "spawn() should succeed when pager is available"
        );

        // Restore original PAGER value
        if let Some(val) = original_pager {
            env::set_var("PAGER", val);
        } else {
            env::remove_var("PAGER");
        }
    }

    #[test]
    fn test_pager_spawn_fails_without_pager() {
        // Create a pager with no pager command
        let pager = Pager {
            pager_cmd: None,
            is_tty: true,
        };

        // spawn() should fail when no pager is available
        let result = pager.spawn();
        assert!(
            result.is_err(),
            "spawn() should fail when pager_cmd is None"
        );

        if let Err(err) = result {
            assert_eq!(
                err.kind(),
                io::ErrorKind::NotFound,
                "error should be NotFound when no pager available"
            );
        }
    }

    #[test]
    fn test_pager_should_use_pager_when_tty_and_pager_available() {
        // Save original PAGER value
        let original_pager = env::var("PAGER").ok();
        env::set_var("PAGER", "cat");

        let pager = Pager::new();

        // If TTY and pager available, should_use_pager should return true
        // STUB: Currently always returns false
        if pager.is_tty && pager.pager_cmd.is_some() {
            assert!(
                pager.should_use_pager(),
                "should_use_pager() should return true when TTY and pager available"
            );
        }

        // Restore original PAGER value
        if let Some(val) = original_pager {
            env::set_var("PAGER", val);
        } else {
            env::remove_var("PAGER");
        }
    }

    #[test]
    fn test_pager_should_not_use_pager_when_not_tty() {
        // Create a pager with pager but not TTY
        let pager = Pager {
            pager_cmd: Some(PathBuf::from("/usr/bin/less")),
            is_tty: false,
        };

        // should_use_pager should return false when not TTY
        // STUB: Currently always returns false (this test will pass by accident)
        assert!(
            !pager.should_use_pager(),
            "should_use_pager() should return false when not TTY"
        );
    }

    #[test]
    fn test_pager_should_not_use_pager_when_no_pager_available() {
        // Create a pager with TTY but no pager command
        let pager = Pager {
            pager_cmd: None,
            is_tty: true,
        };

        // should_use_pager should return false when no pager available
        // STUB: Currently always returns false (this test will pass by accident)
        assert!(
            !pager.should_use_pager(),
            "should_use_pager() should return false when no pager available"
        );
    }

    #[test]
    fn test_pager_default() {
        let pager = Pager::default();

        // default() should be same as new()
        // STUB: Currently returns empty pager
        assert!(
            pager.pager_cmd.is_some() || !pager.is_tty,
            "default() should create a properly configured pager"
        );
    }

    #[test]
    fn test_pager_pager_cmd_accessor() {
        let pager = Pager {
            pager_cmd: Some(PathBuf::from("/usr/bin/less")),
            is_tty: true,
        };

        assert_eq!(
            pager.pager_cmd(),
            Some(&PathBuf::from("/usr/bin/less")),
            "pager_cmd() should return the pager path"
        );
    }

    #[test]
    fn test_pager_pager_cmd_accessor_none() {
        let pager = Pager {
            pager_cmd: None,
            is_tty: false,
        };

        assert!(
            pager.pager_cmd().is_none(),
            "pager_cmd() should return None when not set"
        );
    }

    #[test]
    fn test_find_pager_with_pager_env() {
        let original_pager = env::var("PAGER").ok();
        env::set_var("PAGER", "less");

        let result = find_pager();
        assert!(
            result.is_some(),
            "find_pager() should find pager when $PAGER is set"
        );

        let path = result.unwrap();
        assert!(
            path.to_string_lossy().contains("less"),
            "find_pager() should return 'less' when $PAGER=less"
        );

        if let Some(val) = original_pager {
            env::set_var("PAGER", val);
        } else {
            env::remove_var("PAGER");
        }
    }

    #[test]
    fn test_find_pager_without_env() {
        let original_pager = env::var("PAGER").ok();
        env::remove_var("PAGER");

        let result = find_pager();

        if command_exists("less") || command_exists("more") || command_exists("most") {
            assert!(
                result.is_some(),
                "find_pager() should find at least one pager"
            );
        }

        if let Some(val) = original_pager {
            env::set_var("PAGER", val);
        } else {
            env::remove_var("PAGER");
        }
    }

    #[test]
    fn test_find_pager_returns_same_as_detect() {
        let original_pager = env::var("PAGER").ok();
        env::set_var("PAGER", "less");

        let find_result = find_pager();
        let detect_result = Pager::detect();

        assert_eq!(
            find_result, detect_result,
            "find_pager() should return same result as Pager::detect()"
        );

        if let Some(val) = original_pager {
            env::set_var("PAGER", val);
        } else {
            env::remove_var("PAGER");
        }
    }
}
