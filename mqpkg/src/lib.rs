// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::clone::Clone;
use std::collections::HashMap;

use vfs::VfsPath;

use crate::pkgdb::transaction;
use crate::types::{RequestedPackages, SolverSolution};

pub use crate::config::Config;
pub use crate::errors::{InstallerError, SolverError};
pub use crate::types::PackageSpecifier;

pub(crate) mod types;

mod config;
mod errors;
mod pkgdb;
mod repository;
mod resolver;

type Result<T, E = InstallerError> = core::result::Result<T, E>;

pub struct Installer {
    config: config::Config,
    db: pkgdb::Database,
}

impl Installer {
    pub fn new(config: config::Config, fs: VfsPath, rid: &str) -> Result<Installer> {
        // We're using MD5 here because it's short and fast, we're not using
        // this in a security sensitive aspect.
        let id = format!("{:x}", md5::compute(rid));
        let db = pkgdb::Database::new(fs, id)?;

        Ok(Installer { config, db })
    }

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

impl Installer {
    fn resolve(&self, requested: RequestedPackages) -> Result<SolverSolution> {
        let repository = repository::Repository::new()?.fetch(self.config.repositories())?;
        let solver = resolver::Solver::new(repository);
        let solution = solver.resolve(requested)?;

        Ok(solution)
    }
}
