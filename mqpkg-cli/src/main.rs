// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use vfs::{PhysicalFS, VfsPath};

use mqpkg::{find_config_dir, Config, CONFIG_FILENAME};

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
        Some(target) => {
            let root: VfsPath = PhysicalFS::new(PathBuf::from(&target)).into();
            Config::load(root).with_context(|| format!("Invalid target directory '{}'", target))?
        }
        None => {
            let root: VfsPath = PhysicalFS::new(find_config_dir(std::env::current_dir()?)?).into();
            Config::load(root).with_context(|| {
                format!(
                    "Unable to find '{}' in current directory or parents",
                    CONFIG_FILENAME
                )
            })?
        }
    };

    match &cli.command {
        Commands::Install {} => Ok(()),
        Commands::Uninstall {} => Ok(()),
        Commands::Upgrade {} => Ok(()),
    }
}
