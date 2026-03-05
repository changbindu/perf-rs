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
            commands::report::execute(input.as_deref(), &format, sort.as_deref(), top)?;
        }
        Commands::Script {
            input,
            format,
            callchain,
        } => {
            commands::script::execute(input.as_deref(), &format, callchain)?;
        }
    }

    Ok(())
}
