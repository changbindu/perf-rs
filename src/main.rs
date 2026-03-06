mod arch;
mod cli;
mod commands;
mod core;
mod error;
mod symbols;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use log::{debug, error, info};

fn main() -> Result<()> {
    let args = Cli::parse();

    // Initialize logging based on verbose flag
    env_logger::Builder::new()
        .filter_level(if args.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Warn
        })
        .init();

    info!("Starting perf-rs with verbose={}", args.verbose);
    debug!("Command: {:?}", args.command);

    if let Err(e) = run_command(&args) {
        error!("Command failed: {}", e);

        // Print error chain for better diagnostics
        let mut source = e.source();
        while let Some(cause) = source {
            eprintln!("  Caused by: {}", cause);
            source = cause.source();
        }

        // Check if it's a permission error and provide helpful context
        let err_string = e.to_string().to_lowercase();
        if err_string.contains("permission") || err_string.contains("privilege") {
            eprintln!();
            eprintln!("Hint: Performance monitoring requires elevated privileges.");
            eprintln!("      Try running with sudo or check your capabilities.");
            eprintln!("      Run with --verbose for more details.");
        }

        std::process::exit(1);
    }

    info!("Command completed successfully");
    Ok(())
}

fn run_command(args: &Cli) -> Result<()> {
    match &args.command {
        Commands::List { filter, detailed } => {
            debug!(
                "Executing list command: filter={:?}, detailed={}",
                filter, detailed
            );
            commands::list::execute(filter.as_deref(), *detailed)
                .context("Failed to list performance events")?;
        }
        Commands::Stat {
            pid,
            event,
            all_cpus,
            cpu,
            per_cpu,
            command,
        } => {
            debug!(
                "Executing stat command: pid={:?}, event={:?}, all_cpus={}, cpu={:?}, per_cpu={}, command={:?}",
                pid, event, all_cpus, cpu, per_cpu, command
            );
            commands::stat::execute(
                *pid,
                event.as_deref(),
                *all_cpus,
                cpu.as_deref(),
                *per_cpu,
                command,
            )
            .context("Failed to collect performance statistics")?;
        }
        Commands::Record {
            pid,
            all_cpus,
            cpu,
            output,
            event,
            frequency,
            period,
            command,
        } => {
            debug!(
                "Executing record command: pid={:?}, all_cpus={}, cpu={:?}, output={:?}, event={:?}, frequency={:?}, period={:?}, command={:?}",
                pid, all_cpus, cpu, output, event, frequency, period, command
            );
            commands::record::execute(
                *pid,
                *all_cpus,
                cpu.as_deref(),
                output.as_deref(),
                event.as_deref(),
                *frequency,
                *period,
                command,
            )
            .context("Failed to record performance data")?;
        }
        Commands::Report {
            input,
            format,
            sort,
            top,
        } => {
            debug!(
                "Executing report command: input={:?}, format={}, sort={:?}, top={:?}",
                input, format, sort, top
            );
            commands::report::execute(input.as_deref(), format, sort.as_deref(), *top)
                .context("Failed to generate performance report")?;
        }
        Commands::Script {
            input,
            format,
            callchain,
        } => {
            debug!(
                "Executing script command: input={:?}, format={}, callchain={}",
                input, format, callchain
            );
            commands::script::execute(input.as_deref(), format, *callchain)
                .context("Failed to generate script output")?;
        }
    }

    Ok(())
}
