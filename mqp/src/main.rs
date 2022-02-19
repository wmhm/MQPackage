// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.
use clap::{Parser, Subcommand};

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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Install {} => {}
        Commands::Uninstall {} => {}
        Commands::Upgrade {} => {}
    }
}
