// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use indicatif::ProgressBar;
use vfs::{PhysicalFS, VfsPath};

use mqpkg::{Config, Installer, InstallerError, PackageSpecifier, SolverError};

#[derive(Parser, Debug)]
#[clap(version)]
struct Cli {
    #[clap(global = true, short, long)]
    target: Option<Utf8PathBuf>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
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
    let root = match cli.target {
        Some(target) => canonicalize(target)?,
        None => Config::find(current_dir()?).with_context(|| {
            format!(
                "unable to find '{}' in current directory or parents",
                Config::filename()
            )
        })?,
    };

    // Build our VFS, Config, and Installer objects, and a HashMap to hold our
    // progress bars.
    let fs: VfsPath = PhysicalFS::new(PathBuf::from(&root)).into();
    let config =
        Config::load(&fs).with_context(|| format!("invalid target directory '{}'", root))?;
    let mut pkg = Installer::new(config, fs, root.as_str())
        .with_context(|| format!("could not initialize in '{}'", root))?;

    // Setup our progress callbacks.
    pkg.with_progress_start(ProgressBar::new);
    pkg.with_progress_update(|bar, delta| bar.inc(delta));
    pkg.with_progress_finish(|bar| bar.finish_and_clear());

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
