// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use vfs::{PhysicalFS, VfsPath};

use mqpkg::errors::{MQPkgError, SolverError};
use mqpkg::{Config, MQPkg, PackageSpecifier};

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
    let fs: VfsPath = PhysicalFS::new(PathBuf::from(&root)).into();
    let config =
        Config::load(&fs).with_context(|| format!("invalid target directory '{}'", root))?;
    let mut pkg = MQPkg::new(config, fs, root.as_str())
        .with_context(|| format!("could not initialize in '{}'", root))?;

    match &cli.command {
        Commands::Install { packages } => match pkg.install(packages) {
            Ok(v) => Ok(v),
            Err(MQPkgError::ResolverError(SolverError::NoSolution(mut dt))) => {
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
