// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr, PickFirst};
use thiserror::Error;
use url::Url;
use vfs::VfsPath;

pub const CONFIG_FILENAME: &str = "mqpkg.yml";

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("no configuration file")]
    NoConfig { source: vfs::VfsError },

    #[error("invalid configuration")]
    InvalidConfig { source: serde_yaml::Error },

    #[error("invalid url")]
    InvalidURL { source: url::ParseError },

    #[error("unable to traverse directory")]
    DirectoryTraversalError { source: vfs::VfsError },

    #[error("unable to locate a valid directory")]
    NoTargetDirectoryFound,
}

#[derive(Deserialize, Debug)]
struct Repository {
    #[serde(rename = "url")]
    _url: Url,
}

impl FromStr for Repository {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::from_str(s).map_err(|source| ConfigError::InvalidURL { source })?;

        Ok(Repository { _url: url })
    }
}

#[serde_with::serde_as]
#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(rename = "repositories")]
    #[serde_as(as = "Vec<PickFirst<(_, DisplayFromStr)>>")]
    _repositories: Vec<Repository>,
}

impl Config {
    pub fn load(root: &VfsPath) -> Result<Config, ConfigError> {
        let file = root
            .join(CONFIG_FILENAME)
            .map_err(|source| ConfigError::NoConfig { source })?
            .open_file()
            .map_err(|source| ConfigError::NoConfig { source })?;
        let config: Config = serde_yaml::from_reader(file)
            .map_err(|source| ConfigError::InvalidConfig { source })?;

        Ok(config)
    }
}

pub fn find_config_dir<P>(path: P) -> Result<PathBuf, ConfigError>
where
    P: Into<PathBuf>,
{
    let mut path = path.into();
    loop {
        path.push(CONFIG_FILENAME);
        if path.is_file() {
            assert!(path.pop());
            break Ok(path);
        }

        // Remove the filename, and the parent, and
        // if that's not successful, it's an error.
        if !(path.pop() && path.pop()) {
            return Err(ConfigError::NoTargetDirectoryFound);
        }
    }
}
