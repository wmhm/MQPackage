// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::path::{Path, PathBuf};
use thiserror::Error;

const REPOSITORY_FILENAME: &str = "repositories.yml";

#[derive(Error, Debug)]
pub enum MQPackageError {
    #[error("unable to locate a valid directory")]
    NoTargetDirectoryFound,
}

#[derive(Debug)]
pub struct Config {
    target: PathBuf,
}

impl Config {
    pub fn target(&self) -> &Path {
        self.target.as_path()
    }
}

impl Config {
    pub fn load<P>(path: P) -> Result<Config, MQPackageError>
    where
        P: Into<PathBuf>,
    {
        Ok(Config {
            target: path.into(),
        })
    }

    pub fn find<P>(path: P) -> Result<Config, MQPackageError>
    where
        P: Into<PathBuf>,
    {
        let mut path = path.into();
        let target = loop {
            path.push(REPOSITORY_FILENAME);
            if path.is_file() {
                assert!(path.pop());
                break path;
            }

            // Remove the filename, and the parent, and
            // if that's not successful, it's an error.
            if !(path.pop() && path.pop()) {
                return Err(MQPackageError::NoTargetDirectoryFound);
            }
        };

        Config::load(target)
    }
}
