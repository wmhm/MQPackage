// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::clone::Clone;
use std::collections::HashMap;

use console::{style, Emoji};
use vfs::VfsPath;

use crate::pkgdb::transaction;
use crate::progress::Progress;
use crate::types::{RequestedPackages, SolverSolution};

pub use crate::config::Config;
pub use crate::errors::{InstallerError, SolverError};
pub use crate::types::PackageSpecifier;

pub(crate) mod progress;
pub(crate) mod types;

mod config;
mod errors;
mod pkgdb;
mod repository;
mod resolver;

const NSTEPS: u8 = 2;

static OFFICE_PAPER: Emoji<'_, '_> = Emoji("📄  ", "");
static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");

type Result<T, E = InstallerError> = core::result::Result<T, E>;

pub struct Installer<'p, T> {
    config: config::Config,
    db: pkgdb::Database,
    progress: Progress<'p, T>,
    console: Option<Box<dyn Fn(&str) + 'p>>,
}

impl<'p, T> Installer<'p, T> {
    pub fn new(config: config::Config, fs: VfsPath, rid: &str) -> Result<Installer<T>> {
        // We're using MD5 here because it's short and fast, we're not using
        // this in a security sensitive aspect.
        let id = format!("{:x}", md5::compute(rid));
        let db = pkgdb::Database::new(fs, id)?;

        Ok(Installer {
            config,
            db,
            progress: Progress::new(),
            console: None,
        })
    }

    pub fn with_console(&mut self, cb: impl Fn(&str) + 'p) {
        self.console = Some(Box::new(cb))
    }

    pub fn with_progress_start(&mut self, cb: impl FnMut(u64) -> T + 'p) {
        self.progress.with_progress_start(Box::new(cb))
    }

    pub fn with_progress_spinner(&mut self, cb: impl FnMut(&'static str) -> T + 'p) {
        self.progress.with_progress_spinner(Box::new(cb))
    }

    pub fn with_progress_update(&mut self, cb: impl FnMut(&T, u64) + 'p) {
        self.progress.with_progress_update(Box::new(cb))
    }

    pub fn with_progress_finish(&mut self, cb: impl FnMut(&T) + 'p) {
        self.progress.with_progress_finish(Box::new(cb))
    }
}

impl<'p, T> Installer<'p, T> {
    pub fn install(&mut self, packages: &[PackageSpecifier]) -> Result<()> {
        transaction!(self.db, {
            // Add all of the packages being requested to the set of all requested packages.
            for package in packages {
                self.db.add(package)?;
            }

            // Get all of the requested packages, we need this to ensure that this install
            // doesn't invalidate any of the version requirements of the already requested
            // packages.
            let mut requested = HashMap::new();
            for req in self.db.requested()?.values() {
                requested.insert(req.name.clone(), req.version.clone());
            }

            // Resolve all of our requirements to a full set of packages that we should install
            let _solution = self.resolve(requested)?;
        });

        Ok(())
    }
}

impl<'p, T> Installer<'p, T> {
    fn console<S: AsRef<str>>(&self, msg: S) {
        if let Some(cb) = &self.console {
            (cb)(msg.as_ref());
        }
    }

    fn resolve(&self, requested: RequestedPackages) -> Result<SolverSolution> {
        let bar = self
            .progress
            .bar(self.config.repositories().len().try_into().unwrap());
        let repository =
            repository::Repository::new()?.fetch(self.config.repositories(), || bar.update(1))?;
        bar.finish();

        self.console(step(1, OFFICE_PAPER, "Fetched package metadata"));

        let spinner = self.progress.spinner("Resolving dependencies");
        let solver = resolver::Solver::new(repository);
        let solution = solver.resolve(requested, || spinner.update(1))?;
        spinner.finish();
        self.console(step(2, LOOKING_GLASS, "Resolved dependencies"));

        Ok(solution)
    }
}

fn step(n: u8, emoji: Emoji, msg: &str) -> String {
    let prefix = style(format!("[{n}/{NSTEPS}]")).bold().dim();
    format!("{prefix} {emoji}{msg}")
}
