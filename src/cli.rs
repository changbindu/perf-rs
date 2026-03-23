//! CLI interface for perf-rs.
//!
//! This module defines the command-line interface using clap derive macros.

use clap::{Parser, Subcommand, ValueEnum};

/// Call graph (stack unwinding) method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CallGraphMethod {
    /// Frame pointer unwinding (default).
    ///
    /// Uses frame pointers to walk the call stack. Requires binaries
    /// compiled with frame pointers enabled (-fno-omit-frame-pointer).
    Fp,

    /// DWARF unwinding using .eh_frame/.debug_frame sections.
    ///
    /// Uses DWARF debug information for more accurate stack unwinding.
    /// Works with binaries compiled without frame pointers.
    Dwarf,
}

/// Linux performance monitoring tool in Rust.
#[derive(Parser, Debug)]
#[command(name = "perf-rs")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Disable pagination for long output
    #[arg(long, global = true)]
    pub no_pager: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for perf-rs.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List available performance events.
    ///
    /// Displays a list of available hardware and software performance events
    /// that can be monitored using the stat and record commands.
    List {
        /// Filter events by name pattern (substring match)
        #[arg(short, long)]
        filter: Option<String>,

        /// Show detailed event information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Run a command and gather performance statistics.
    ///
    /// Counts performance events for a command or process and displays
    /// statistics after completion.
    Stat {
        /// Process ID to monitor (mutually exclusive with command)
        #[arg(short, long, value_name = "PID")]
        pid: Option<u32>,

        /// Performance events to monitor (comma-separated)
        #[arg(short, long, value_name = "EVENTS")]
        event: Option<String>,

        /// Monitor system-wide across all CPUs
        #[arg(short = 'a', long)]
        all_cpus: bool,

        /// Monitor specific CPUs (comma-separated list or range, e.g., '0,2,4-6')
        #[arg(short = 'C', long, value_name = "CPUS", conflicts_with = "all_cpus")]
        cpu: Option<String>,

        /// Show per-CPU breakdown of statistics (requires -a or -C)
        ///
        /// Displays counter values for each CPU separately in a table format,
        /// sorted by CPU ID then event name. Includes overhead percentages
        /// relative to the total for each event across all monitored CPUs.
        #[arg(long)]
        per_cpu: bool,

        /// Command to execute (mutually exclusive with --pid)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Record samples for a command or process.
    ///
    /// Samples performance events and writes them to a file for later
    /// analysis with the report command.
    Record {
        /// Process ID to monitor (mutually exclusive with command)
        #[arg(short, long, value_name = "PID")]
        pid: Option<u32>,

        /// System-wide collection from all CPUs (mutually exclusive with --cpu)
        #[arg(short = 'a', long, conflicts_with = "cpu")]
        all_cpus: bool,

        /// List of CPUs to monitor (e.g., 0,2,4 or 0-3)
        #[arg(short = 'C', long, value_name = "CPUS", conflicts_with = "all_cpus")]
        cpu: Option<String>,

        /// Output file for recorded data (default: perf.data)
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Performance events to monitor (comma-separated)
        #[arg(short, long, value_name = "EVENTS")]
        event: Option<String>,

        /// Sample frequency in Hz (mutually exclusive with --period)
        #[arg(short, long, value_name = "HZ")]
        frequency: Option<u64>,

        /// Sample period (number of events between samples)
        #[arg(short = 'P', long, value_name = "N", conflicts_with = "frequency")]
        period: Option<u64>,

        /// Enable call graph (stack trace) recording
        ///
        /// When enabled, each sample will include a stack trace showing
        /// the function call chain leading to the sampled instruction.
        ///
        /// Methods:
        ///   fp     - Frame pointer unwinding (default, requires -fno-omit-frame-pointer)
        ///   dwarf  - DWARF unwinding using debug info (works without frame pointers)
        ///
        /// Examples:
        ///   -g                    # Enable with default method (fp)
        ///   --call-graph=fp       # Explicit frame pointer unwinding
        ///   --call-graph=dwarf    # DWARF-based unwinding
        #[arg(short = 'g', long, value_name = "METHOD", num_args = 0..=1, default_missing_value = "fp")]
        call_graph: Option<CallGraphMethod>,

        /// Maximum stack depth for call graph recording (default: 127)
        #[arg(long, value_name = "N", default_value = "127")]
        stack_size: u16,

        /// Command to execute (mutually exclusive with --pid)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Analyze recorded performance data.
    ///
    /// Reads perf.data file and generates a report showing where
    /// time was spent or which functions had the most events.
    Report {
        /// Input file to analyze (default: perf.data)
        #[arg(short, long, value_name = "FILE")]
        input: Option<String>,

        /// Output format (text, json, tui)
        #[arg(short, long, value_name = "FORMAT", default_value = "text")]
        format: String,

        /// Sort by this field (overhead, sample, period)
        #[arg(short, long, value_name = "FIELD")]
        sort: Option<String>,

        /// Show n most expensive functions (default: show all)
        #[arg(short, long, value_name = "N")]
        top: Option<usize>,
    },

    /// Dump trace data from recorded file.
    ///
    /// Reads perf.data file and displays the raw trace data in
    /// a human-readable format.
    Script {
        /// Input file to read (default: perf.data)
        #[arg(short, long, value_name = "FILE")]
        input: Option<String>,

        /// Output format (text, json)
        #[arg(short, long, value_name = "FORMAT", default_value = "text")]
        format: String,

        /// Do not show call chains (shown by default)
        #[arg(short = 'G', long)]
        no_call_graph: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(["perf-rs", "--verbose", "list"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert!(cli.verbose);
        assert!(matches!(cli.command, Commands::List { .. }));
    }

    #[test]
    fn test_no_pager_flag_parsing() {
        let cli = Cli::try_parse_from(["perf-rs", "--no-pager", "list"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert!(cli.no_pager);
        assert!(matches!(cli.command, Commands::List { .. }));
    }

    #[test]
    fn test_stat_command_parsing() {
        let cli = Cli::try_parse_from(["perf-rs", "stat", "--pid", "1234"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Stat { pid, .. } => assert_eq!(pid, Some(1234)),
            _ => panic!("Expected Stat command"),
        }
    }

    #[test]
    fn test_record_command_parsing() {
        let cli = Cli::try_parse_from([
            "perf-rs",
            "record",
            "--output",
            "custom.data",
            "--frequency",
            "99",
            "ls",
            "-la",
        ]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Record {
                output,
                frequency,
                command,
                ..
            } => {
                assert_eq!(output, Some("custom.data".to_string()));
                assert_eq!(frequency, Some(99));
                assert_eq!(command, vec!["ls", "-la"]);
            }
            _ => panic!("Expected Record command"),
        }
    }

    #[test]
    fn test_report_command_parsing() {
        let cli = Cli::try_parse_from([
            "perf-rs",
            "report",
            "--input",
            "perf.data",
            "--format",
            "json",
            "--top",
            "10",
        ]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Report {
                input, format, top, ..
            } => {
                assert_eq!(input, Some("perf.data".to_string()));
                assert_eq!(format, "json");
                assert_eq!(top, Some(10));
            }
            _ => panic!("Expected Report command"),
        }
    }

    #[test]
    fn test_script_command_parsing() {
        let cli = Cli::try_parse_from(["perf-rs", "script", "--no-call-graph"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Script { no_call_graph, .. } => assert!(no_call_graph),
            _ => panic!("Expected Script command"),
        }
    }

    #[test]
    fn test_call_graph_option_parsing() {
        // Test -g shorthand (defaults to Fp)
        let cli = Cli::try_parse_from(["perf-rs", "record", "-g", "--", "ls"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Record { call_graph, .. } => {
                assert_eq!(call_graph, Some(CallGraphMethod::Fp));
            }
            _ => panic!("Expected Record command"),
        }

        // Test --call-graph=fp
        let cli = Cli::try_parse_from(["perf-rs", "record", "--call-graph=fp", "--", "ls"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Record { call_graph, .. } => {
                assert_eq!(call_graph, Some(CallGraphMethod::Fp));
            }
            _ => panic!("Expected Record command"),
        }

        // Test --call-graph=dwarf
        let cli = Cli::try_parse_from(["perf-rs", "record", "--call-graph=dwarf", "--", "ls"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Record { call_graph, .. } => {
                assert_eq!(call_graph, Some(CallGraphMethod::Dwarf));
            }
            _ => panic!("Expected Record command"),
        }

        // Test invalid value
        let cli = Cli::try_parse_from(["perf-rs", "record", "--call-graph=invalid", "--", "ls"]);
        assert!(cli.is_err());

        // Test no call graph option
        let cli = Cli::try_parse_from(["perf-rs", "record", "--", "ls"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Record { call_graph, .. } => {
                assert_eq!(call_graph, None);
            }
            _ => panic!("Expected Record command"),
        }
    }

    #[test]
    fn test_stack_size_option() {
        // Test default stack size
        let cli = Cli::try_parse_from(["perf-rs", "record", "-g", "--", "ls"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Record { stack_size, .. } => {
                assert_eq!(stack_size, 127);
            }
            _ => panic!("Expected Record command"),
        }

        // Test custom stack size
        let cli =
            Cli::try_parse_from(["perf-rs", "record", "-g", "--stack-size", "50", "--", "ls"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Record { stack_size, .. } => {
                assert_eq!(stack_size, 50);
            }
            _ => panic!("Expected Record command"),
        }
    }
}
