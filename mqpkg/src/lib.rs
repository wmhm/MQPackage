// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use thiserror::Error as BaseError;

pub use self::config::Config;

mod config;

#[derive(BaseError, Debug)]
pub enum Error {
    #[error("no configuration file")]
    NoConfig {
        #[from]
        source: std::io::Error,
    },

    #[error("invalid configuration")]
    InvalidConfig {
        #[from]
        source: serde_yaml::Error,
    },

    #[error("invalid url")]
    InvalidURL {
        #[from]
        source: url::ParseError,
    },

    #[error("unable to locate a valid directory")]
    NoTargetDirectoryFound,
}
