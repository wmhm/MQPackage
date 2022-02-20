// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr, PickFirst};
use url::Url;

use super::Error;

const CONFIG_FILENAME: &str = "mqpkg.yml";

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
    #[serde(skip)]
    target: PathBuf,

    #[serde(rename = "repositories")]
    #[serde_as(as = "Vec<PickFirst<(_, DisplayFromStr)>>")]
    _repositories: Vec<Repository>,
}

impl Config {
    pub fn target(&self) -> &Path {
        self.target.as_path()
    }
}

impl Config {
    pub fn load<P>(path: P) -> Result<Config, Error>
    where
        P: AsRef<Path>,
    {
        let target = dunce::canonicalize(path)?;
        let configfile = File::open(target.join(CONFIG_FILENAME))
            .map_err(|source| Error::NoConfig { source })?;
        let config: Config = serde_yaml::from_reader(configfile)?;

        Ok(Config { target, ..config })
    }

    pub fn find<P>(path: P) -> Result<Config, Error>
    where
        P: Into<PathBuf>,
    {
        let mut path = path.into();
        let target = loop {
            path.push(CONFIG_FILENAME);
            if path.is_file() {
                assert!(path.pop());
                break path;
            }

            // Remove the filename, and the parent, and
            // if that's not successful, it's an error.
            if !(path.pop() && path.pop()) {
                return Err(Error::NoTargetDirectoryFound);
            }
        };

        Config::load(target)
    }
}
