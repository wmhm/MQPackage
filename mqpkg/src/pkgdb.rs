// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::collections::HashMap;
use std::default::Default;
use std::mem::drop;

use semver::VersionReq;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vfs::VfsPath;

use crate::pkgdb::transactions::{Transaction, TransactionManager};
use crate::{PackageName, PackageSpecifier};

pub mod transactions;

const PKGDB_DIR: &str = "pkgdb";
const STATE_FILE: &str = "state.yml";

#[derive(Error, Debug)]
pub enum DBError {
    #[error("could not access the pkgdb")]
    PathUnavailable(#[from] vfs::VfsError),

    #[error("could not parse state.yml")]
    InvalidState { source: serde_yaml::Error },

    #[error("could not initiate transaction")]
    TransactionError(#[from] transactions::TransactionError),
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

impl State {
    fn load(fs: &VfsPath) -> DBResult<State> {
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

    fn save(&self, fs: &VfsPath) -> DBResult<()> {
        ensure_dir(&pkgdb_path(fs)?)?;

        let file = state_path(fs)?.create_file()?;
        serde_yaml::to_writer(file, self).map_err(|source| DBError::InvalidState { source })?;
        Ok(())
    }
}

type DBResult<T> = Result<T, DBError>;

pub struct Database {
    id: String,
    fs: VfsPath,
    state: State,
}

impl Database {
    pub fn new(fs: VfsPath, id: String) -> DBResult<Database> {
        let state = State::load(&fs)?;
        Ok(Database { id, fs, state })
    }

    pub fn add(&mut self, package: &PackageSpecifier) -> DBResult<()> {
        self.state.requested.insert(
            package.name.clone(),
            PackageRequest {
                name: package.name.clone(),
                version: package.version.clone(),
            },
        );
        Ok(())
    }

    pub(crate) fn transaction(&self) -> DBResult<TransactionManager> {
        Ok(TransactionManager::new(&self.id)?)
    }

    pub(crate) fn begin<'r>(&self, txnm: &'r TransactionManager) -> DBResult<Transaction<'r>> {
        Ok(txnm.begin()?)
    }

    pub(crate) fn commit(&self, txn: Transaction) -> DBResult<()> {
        self.state.save(&self.fs)?;

        // Drop our transaction, which unlocks everything, and ensures that
        // our transaction is open to everyone to use again. We could just
        // let the fact that txn moved into commit auto drop this, but this
        // documents our intent more, rather than just having an unused
        // parameter.
        drop(txn);

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
