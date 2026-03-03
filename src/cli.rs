//! CLI interface for perf-rs.
//!
//! This module defines the command-line interface using clap derive macros.

use clap::{Parser, Subcommand};

/// Linux performance monitoring tool in Rust.
#[derive(Parser, Debug)]
#[command(name = "perf-rs")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

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

        /// Show call chains
        #[arg(short, long)]
        callchain: bool,
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
        let cli = Cli::try_parse_from(["perf-rs", "script", "--callchain"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Commands::Script { callchain, .. } => assert!(callchain),
            _ => panic!("Expected Script command"),
        }
    }
}
