// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use vfs::{PhysicalFS, VfsPath};

use mqpkg::{Config, Installer, InstallerError, PackageSpecifier, SolverError};

mod logging;

const LOGNAME: &str = "mqpkg";

#[derive(Debug, Parser)]
#[clap(version)]
struct Cli {
    #[clap(flatten)]
    verbose: Verbosity<WarnLevel>,

    #[clap(global = true, short, long)]
    target: Option<Utf8PathBuf>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Install {
        #[clap(required = true)]
        packages: Vec<PackageSpecifier>,
    },
    Uninstall {},
    Upgrade {},
}

fn main() -> Result<()> {
    // Parse our CLI parameters.
    let cli = Cli::parse();

    // Setup a few items for our progress bar handling
    let style = ProgressStyle::default_bar().progress_chars("█▇▆▅▄▃▂▁  ");
    let bars = Arc::new(Mutex::new(Vec::new()));

    // Setup our logging.
    let render_bars =
        cli.verbose.log_level().or(Some(log::Level::Error)).unwrap() >= log::Level::Warn;
    logging::setup(cli.verbose.log_level_filter(), bars.clone());

    // Build our VFS, Config, and Installer objects, and a HashMap to hold our
    // progress bars.
    let root = match cli.target {
        Some(target) => canonicalize(target)?,
        None => Config::find(current_dir()?).with_context(|| {
            format!(
                "unable to find '{}' in current directory or parents",
                Config::filename()
            )
        })?,
    };
    info!(target: LOGNAME, "using root directory: '{}'", root);
    let fs: VfsPath = PhysicalFS::new(PathBuf::from(&root)).into();
    let config =
        Config::load(&fs).with_context(|| format!("invalid target directory '{}'", root))?;
    let mut pkg = Installer::new(config, fs, root.as_str())
        .with_context(|| format!("could not initialize in '{}'", root))?;

    // Setup our progress callbacks.
    if render_bars {
        pkg.with_progress_start(|len| {
            let mut b = bars.lock().unwrap();
            let bar = ProgressBar::new(len);
            bar.set_style(style.clone());
            b.push(bar.downgrade());
            bar
        });
        pkg.with_progress_update(|bar, delta| bar.inc(delta));
        pkg.with_progress_finish(|bar| bar.finish_and_clear());
    }

    // Actually dispatch to our commands.
    match &cli.command {
        Commands::Install { packages } => match pkg.install(packages) {
            Ok(v) => Ok(v),
            Err(InstallerError::ResolverError(SolverError::NoSolution(mut dt))) => {
                dt.collapse_no_versions();
                Err(SolverError::humanized(
                    "unable to resolve packages to a set that satisfies all requirements",
                    *dt,
                )
                .into())
            }
            Err(err) => Err(err.into()),
        },
        _ => Err(anyhow!("command not implemented")),
    }
}

fn canonicalize<P: AsRef<Path>>(path: P) -> Result<Utf8PathBuf> {
    Ok(Utf8PathBuf::try_from(dunce::canonicalize(path)?)?)
}

fn current_dir() -> Result<Utf8PathBuf> {
    canonicalize(std::env::current_dir()?)
}
