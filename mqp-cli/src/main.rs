// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use mqp::Config;

#[derive(Parser, Debug)]
#[clap(version)]
struct Cli {
    #[clap(global = true, short, long)]
    target: Option<String>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Install {},
    Uninstall {},
    Upgrade {},
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let _config = match cli.target {
        Some(target) => Config::load(&target)
            .with_context(|| format!("Invalid target directory '{}'", target))?,
        None => {
            let cur = std::env::current_dir()?;
            mqp::Config::find(&cur)
                .with_context(|| format!("Unable to find target dir from '{}'", cur.display()))?
        }
    };

    match &cli.command {
        Commands::Install {} => Ok(()),
        Commands::Uninstall {} => Ok(()),
        Commands::Upgrade {} => Ok(()),
    }
}
