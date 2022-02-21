// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::collections::HashMap;
use std::default::Default;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use vfs::VfsPath;

use super::PackageName;

const PKGDB_DIR: &str = "pkgdb";
const STATE_FILE: &str = "state.yml";

#[derive(Error, Debug)]
pub enum PkgDBError {
    #[error("could not access the pkgdb")]
    PathUnavailable(#[from] vfs::VfsError),

    #[error("could not parse state.yml")]
    InvalidState { source: serde_yaml::Error },
}

#[derive(Serialize, Deserialize, Debug)]
struct PackageRequest {
    name: PackageName,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
struct State {
    requested: HashMap<PackageName, PackageRequest>,
}

type PkgDBResult<T> = Result<T, PkgDBError>;

pub struct PkgDB {
    fs: VfsPath,
    state: State,
}

impl PkgDB {
    pub fn new(fs: VfsPath) -> PkgDBResult<PkgDB> {
        let state = PkgDB::load_state(&fs)?;
        Ok(PkgDB { fs, state })
    }

    pub fn request_package(&mut self, package: &PackageName) -> PkgDBResult<()> {
        let request = match self.state.requested.remove(package) {
            Some(r) => r,
            None => PackageRequest {
                name: package.clone(),
            },
        };
        self.state.requested.insert(package.clone(), request);
        self.save_state()?;
        Ok(())
    }
}

impl PkgDB {
    fn load_state(fs: &VfsPath) -> PkgDBResult<State> {
        let filename = state_path(fs)?;
        let state: State = if filename.is_file()? {
            serde_yaml::from_reader(filename.open_file()?)
                .map_err(|source| PkgDBError::InvalidState { source })?
        } else {
            State {
                ..Default::default()
            }
        };

        Ok(state)
    }

    fn save_state(&self) -> PkgDBResult<()> {
        ensure_dir(&pkgdb_path(&self.fs)?)?;

        let file = state_path(&self.fs)?.create_file()?;
        serde_yaml::to_writer(file, &self.state)
            .map_err(|source| PkgDBError::InvalidState { source })?;
        Ok(())
    }
}

fn pkgdb_path(fs: &VfsPath) -> PkgDBResult<VfsPath> {
    Ok(fs.join(PKGDB_DIR)?)
}

fn state_path(fs: &VfsPath) -> PkgDBResult<VfsPath> {
    Ok(pkgdb_path(fs)?.join(STATE_FILE)?)
}

fn ensure_dir(path: &VfsPath) -> PkgDBResult<()> {
    if !path.is_dir()? {
        path.create_dir()?;
    }

    Ok(())
}
