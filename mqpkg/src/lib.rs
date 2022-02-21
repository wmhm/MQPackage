// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use thiserror::Error;
use vfs::VfsPath;

pub mod config;
pub mod operations;

mod pkgdb;

#[derive(Error, Debug)]
pub enum EnvironmentError {
    #[error(transparent)]
    PkgDBError(#[from] pkgdb::PkgDBError),
}

pub struct Environment {
    pkgdb: pkgdb::PkgDB,
}

impl Environment {
    pub fn new(_config: config::Config, fs: VfsPath) -> Result<Environment, EnvironmentError> {
        let pkgdb = pkgdb::PkgDB::new(fs.clone())?;

        Ok(Environment { pkgdb })
    }

    pub fn request(&mut self, package: &str) -> Result<(), EnvironmentError> {
        self.pkgdb.request_package(package)?;
        Ok(())
    }
}
