mod arch;
mod cli;
mod commands;
mod core;
mod error;
mod symbols;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::List { filter, detailed } => {
            commands::list::execute(filter.as_deref(), detailed)?;
        }
        Commands::Stat {
            pid,
            event,
            command,
        } => {
            commands::stat::execute(pid, event.as_deref(), &command)?;
        }
        Commands::Record {
            pid,
            output,
            event,
            frequency,
            period,
            command,
        } => {
            commands::record::execute(
                pid,
                output.as_deref(),
                event.as_deref(),
                frequency,
                period,
                &command,
            )?;
        }
        Commands::Report {
            input,
            format,
            sort,
            top,
        } => {
            println!(
                "Report command - input: {:?}, format: {}, sort: {:?}, top: {:?}",
                input, format, sort, top
            );
            // TODO: Implement report subcommand
        }
        Commands::Script {
            input,
            format,
            callchain,
        } => {
            println!(
                "Script command - input: {:?}, format: {}, callchain: {}",
                input, format, callchain
            );
            // TODO: Implement script subcommand
        }
    }

    Ok(())
}
