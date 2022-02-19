// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::fs::File;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;
use url::Url;

const REPOSITORY_FILENAME: &str = "repositories.yml";

#[derive(Error, Debug)]
pub enum MQPackageError {
    #[error("no repository configuration file")]
    NoRepositoryConfig {
        #[from]
        source: std::io::Error,
    },

    #[error("invalid repository configuration")]
    InvalidRepositoryConfig {
        #[from]
        source: serde_yaml::Error,
    },

    #[error("unable to locate a valid directory")]
    NoTargetDirectoryFound,
}

#[derive(Deserialize, Debug)]
pub struct Repository {
    pub repositories: Vec<Url>,
}

#[derive(Debug)]
pub struct Config {
    target: PathBuf,
    repositories: Repository,
}

impl Config {
    pub fn target(&self) -> &Path {
        self.target.as_path()
    }

    pub fn repositories(&self) -> &Repository {
        &self.repositories
    }
}

impl Config {
    pub fn load<P>(path: P) -> Result<Config, MQPackageError>
    where
        P: Into<PathBuf>,
    {
        let path = path.into();
        let repofile = File::open(path.join(REPOSITORY_FILENAME))
            .map_err(|source| MQPackageError::NoRepositoryConfig { source })?;
        let repos: Repository = serde_yaml::from_reader(repofile)?;

        Ok(Config {
            target: path,
            repositories: repos,
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
