// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr, PickFirst};
use url::Url;
use vfs::VfsPath;

use super::Error;

pub const CONFIG_FILENAME: &str = "mqpkg.yml";

#[derive(Deserialize, Debug)]
struct Repository {
    #[serde(rename = "url")]
    _url: Url,
}

impl FromStr for Repository {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::from_str(s).map_err(|source| Error::InvalidURL { source })?;

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
    pub fn load(root: VfsPath) -> Result<Config, Error> {
        let configfile = root
            .join(CONFIG_FILENAME)
            .map_err(|source| Error::NoConfig { source })?
            .open_file()
            .map_err(|source| Error::NoConfig { source })?;
        let config: Config = serde_yaml::from_reader(configfile)
            .map_err(|source| Error::InvalidConfig { source })?;

        Ok(config)
    }
}

pub fn find_config_dir<P>(path: P) -> Result<PathBuf, Error>
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
            return Err(Error::NoTargetDirectoryFound);
        }
    }
}
