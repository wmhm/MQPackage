// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use thiserror::Error as BaseError;

pub use self::config::{find_config_dir, Config, CONFIG_FILENAME};

mod config;

#[derive(BaseError, Debug)]
pub enum Error {
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
