// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use thiserror::Error;

use crate::types::{DerivedResult, PackageName, Version};

#[derive(Error, Debug)]
pub enum MQPkgError {
    #[error(transparent)]
    DBError(#[from] DBError),

    #[error(transparent)]
    RepositoryError(#[from] RepositoryError),

    #[error(transparent)]
    VersionError(#[from] VersionError),

    #[error("error attempting to resolve dependencies")]
    ResolverError(#[from] SolverError),
}

#[derive(Error, Debug)]
pub enum PackageNameError {
    #[error("names must have at least one character")]
    TooShort,

    #[error("names must begin with an alpha character")]
    NoStartingAlpha { name: String, character: String },

    #[error("names must contain only alphanumeric characters")]
    InvalidCharacter { name: String, character: String },
}

#[derive(Error, Debug)]
pub enum VersionError {
    #[error(transparent)]
    ParseError(#[from] semver::Error),
}

#[derive(Error, Debug)]
pub enum PackageSpecifierError {
    #[error("specifier must have a package name")]
    NoPackageName,

    #[error(transparent)]
    InvalidPackageName(#[from] PackageNameError),

    #[error(transparent)]
    InvalidVersionRequirement(#[from] semver::Error),
}

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

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error(transparent)]
    LockError(#[from] named_lock::Error),
}

#[derive(Error, Debug)]
pub enum DBError {
    #[error("could not access the pkgdb")]
    PathUnavailable(#[from] vfs::VfsError),

    #[error("could not parse state.yml")]
    InvalidState { source: serde_yaml::Error },

    #[error("could not initiate transaction")]
    TransactionError(#[from] TransactionError),

    #[error("no transaction")]
    NoTransaction,
}

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error(transparent)]
    HTTPError(#[from] reqwest::Error),

    #[error("could not parse JSON data")]
    Deserialize(#[from] serde_json::Error),

    #[error("could not access local file")]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum SolverError {
    #[error("No solution")]
    NoSolution(Box<DerivedResult>),

    #[error("Package {dependent} required by {package} {version} depends on the empty set")]
    DependencyOnTheEmptySet {
        /// Package whose dependencies we want.
        package: PackageName,
        /// Version of the package for which we want the dependencies.
        version: Version,
        /// The dependent package that requires us to pick from the empty set.
        dependent: PackageName,
    },

    #[error("{package} {version} depends on itself")]
    SelfDependency {
        /// Package whose dependencies we want.
        package: PackageName,
        /// Version of the package for which we want the dependencies.
        version: Version,
    },

    // PubGrubError has a Failure error, and I'm not sure where it would actually
    // be used at, so we're going to just replicate it ourselves.
    #[error("{0}")]
    Failure(String),

    // These errors shouldn't actually be possible, because our implementation
    // of our dependency provider makes sure of that.
    #[error("impossible error")]
    Impossible,
}
