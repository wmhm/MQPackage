// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::str::FromStr;

use camino::Utf8PathBuf;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr, PickFirst};
use url::Url;
use vfs::VfsPath;

use crate::errors::ConfigError;

pub const CONFIG_FILENAME: &str = "mqpkg.yml";

type Result<T, E = ConfigError> = core::result::Result<T, E>;

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Repository {
    pub(crate) name: String,
    pub(crate) url: Url,
}

impl FromStr for Repository {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = s.to_string();
        let url = Url::from_str(s).map_err(|source| ConfigError::InvalidURL { source })?;

        Ok(Repository { name, url })
    }
}

#[serde_with::serde_as]
#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde_as(as = "Vec<PickFirst<(_, DisplayFromStr)>>")]
    repositories: Vec<Repository>,
}

impl Config {
    pub fn load(root: &VfsPath) -> Result<Config> {
        let file = root
            .join(CONFIG_FILENAME)
            .map_err(|source| ConfigError::NoConfig { source })?
            .open_file()
            .map_err(|source| ConfigError::NoConfig { source })?;
        let config: Config = serde_yaml::from_reader(file)
            .map_err(|source| ConfigError::InvalidConfig { source })?;

        Ok(config)
    }

    pub(crate) fn repositories(&self) -> &[Repository] {
        &self.repositories
    }
}

pub fn find_config_dir<P>(path: P) -> Result<Utf8PathBuf>
where
    P: Into<Utf8PathBuf>,
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
