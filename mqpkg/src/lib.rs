// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::clone::Clone;
use std::collections::HashMap;

use log::info;
use vfs::VfsPath;

use crate::pkgdb::transaction;
use crate::progress::Progress;
use crate::types::{RequestedPackages, SolverSolution};

pub use crate::config::Config;
pub use crate::errors::{InstallerError, SolverError};
pub use crate::types::PackageSpecifier;

pub(crate) mod types;

mod config;
mod errors;
mod pkgdb;
mod progress;
mod repository;
mod resolver;

type Result<T, E = InstallerError> = core::result::Result<T, E>;

pub struct Installer<'p, T> {
    config: config::Config,
    db: pkgdb::Database,
    progress: Progress<'p, T>,
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
        })
    }

    pub fn with_progress_start(&mut self, cb: impl FnMut(u64) -> T + 'p) {
        self.progress.with_progress_start(Box::new(cb))
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

            let bar = self.progress.bar(30);
            for _ in 0..30 {
                std::thread::sleep(std::time::Duration::from_millis(250));
                info!(target: "mqpkg", "test");
                bar.update(1);
            }
            bar.finish();

            let bar = self.progress.bar(30);
            for _ in 0..30 {
                std::thread::sleep(std::time::Duration::from_millis(250));
                info!(target: "mqpkg", "test");
                bar.update(1);
            }
            bar.finish();

            // Resolve all of our requirements to a full set of packages that we should install
            let _solution = self.resolve(requested)?;
        });

        Ok(())
    }
}

impl<'p, T> Installer<'p, T> {
    fn resolve(&self, requested: RequestedPackages) -> Result<SolverSolution> {
        let repository = repository::Repository::new()?.fetch(self.config.repositories())?;
        let solver = resolver::Solver::new(repository);
        let solution = solver.resolve(requested)?;

        Ok(solution)
    }
}
