// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use vfs::{PhysicalFS, VfsPath};

use mqpkg::config::{find_config_dir, Config, CONFIG_FILENAME};

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
    let root: VfsPath = match cli.target {
        Some(target) => PhysicalFS::new(PathBuf::from(target)).into(),
        None => {
            let configdir = find_config_dir(std::env::current_dir()?).with_context(|| {
                format!(
                    "Unable to find '{}' in current directory or parents",
                    CONFIG_FILENAME
                )
            })?;
            PhysicalFS::new(configdir).into()
        }
    };
    let _config = Config::load(&root)
        .with_context(|| format!("Invalid target directory '{}'", root.as_str()))?;

    match &cli.command {
        Commands::Install {} => Ok(()),
        Commands::Uninstall {} => Ok(()),
        Commands::Upgrade {} => Ok(()),
    }
}
