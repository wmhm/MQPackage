// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::collections::HashMap;
use std::default::Default;

use semver::VersionReq;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vfs::VfsPath;

use super::{PackageName, PackageSpecifier};

const PKGDB_DIR: &str = "pkgdb";
const STATE_FILE: &str = "state.yml";

#[derive(Error, Debug)]
pub enum DBError {
    #[error("could not access the pkgdb")]
    PathUnavailable(#[from] vfs::VfsError),

    #[error("could not parse state.yml")]
    InvalidState { source: serde_yaml::Error },
}

#[derive(Serialize, Deserialize, Debug)]
struct PackageRequest {
    name: PackageName,
    version: VersionReq,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
struct State {
    requested: HashMap<PackageName, PackageRequest>,
}

type DBResult<T> = Result<T, DBError>;

pub struct Database {
    fs: VfsPath,
    state: State,
}

impl Database {
    pub fn new(fs: VfsPath) -> DBResult<Database> {
        let state = Database::load_state(&fs)?;
        Ok(Database { fs, state })
    }

    pub fn add(&mut self, package: &PackageSpecifier) -> DBResult<()> {
        self.state.requested.insert(
            package.name.clone(),
            PackageRequest {
                name: package.name.clone(),
                version: package.version.clone(),
            },
        );
        self.save_state()?;
        Ok(())
    }
}

impl Database {
    fn load_state(fs: &VfsPath) -> DBResult<State> {
        let filename = state_path(fs)?;
        let state: State = if filename.is_file()? {
            serde_yaml::from_reader(filename.open_file()?)
                .map_err(|source| DBError::InvalidState { source })?
        } else {
            State {
                ..Default::default()
            }
        };

        Ok(state)
    }

    fn save_state(&self) -> DBResult<()> {
        ensure_dir(&pkgdb_path(&self.fs)?)?;

        let file = state_path(&self.fs)?.create_file()?;
        serde_yaml::to_writer(file, &self.state)
            .map_err(|source| DBError::InvalidState { source })?;
        Ok(())
    }
}

fn pkgdb_path(fs: &VfsPath) -> DBResult<VfsPath> {
    Ok(fs.join(PKGDB_DIR)?)
}

fn state_path(fs: &VfsPath) -> DBResult<VfsPath> {
    Ok(pkgdb_path(fs)?.join(STATE_FILE)?)
}

fn ensure_dir(path: &VfsPath) -> DBResult<()> {
    if !path.is_dir()? {
        path.create_dir()?;
    }

    Ok(())
}
