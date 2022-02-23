// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::clone::Clone;
use std::cmp::{Eq, Ord, PartialEq};
use std::fmt;
use std::hash::Hash;
use std::str::FromStr;

use pubgrub::version::Version as RVersion;
use semver::{Version as SemanticVersion, VersionReq};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vfs::VfsPath;

use crate::pkgdb::transactions::transaction;

pub mod config;

mod pkgdb;
mod repository;
mod resolver;

#[derive(Error, Debug)]
pub enum PackageNameError {
    #[error("names must have at least one character")]
    TooShort,

    #[error("names must begin with an alpha character")]
    NoStartingAlpha { name: String, character: String },

    #[error("names must contain only alphanumeric characters")]
    InvalidCharacter { name: String, character: String },
}

#[derive(Serialize, Deserialize, Clone, Eq, Debug, Hash, PartialEq)]
pub struct PackageName(String);

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PackageName {
    type Err = PackageNameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // Check that the first letter is only alpha, and if we don't have
        // a first letter, then this is invalid anyways.
        if !value.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return match value.chars().next() {
                Some(c) => Err(PackageNameError::NoStartingAlpha {
                    name: value.to_string(),
                    character: c.to_string(),
                }),
                None => Err(PackageNameError::TooShort),
            };
        }

        // Iterate over the rest of our letters, and make sure that they're alphanumeric
        for c in value.chars() {
            if !c.is_ascii_alphanumeric() {
                return Err(PackageNameError::InvalidCharacter {
                    name: value.to_string(),
                    character: c.to_string(),
                });
            }
        }

        Ok(PackageName(value.to_ascii_lowercase()))
    }
}

#[derive(Error, Debug)]
pub enum VersionError {
    #[error(transparent)]
    ParseError(#[from] semver::Error),
}

#[derive(Deserialize, Debug, Clone, Ord, Eq, PartialEq, PartialOrd, Hash)]
struct Version(SemanticVersion);

impl Version {
    fn parse(value: &str) -> Result<Version, VersionError> {
        Version::from_str(value)
    }
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Version(SemanticVersion::parse(value)?))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl RVersion for Version {
    fn lowest() -> Version {
        Version(SemanticVersion::new(0, 0, 0))
    }

    fn bump(&self) -> Version {
        Version(SemanticVersion::new(
            self.0.major,
            self.0.minor,
            self.0.patch + 1,
        ))
    }
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

#[derive(Serialize, Deserialize, Clone, Eq, Debug, Hash, PartialEq)]
pub struct PackageSpecifier {
    name: PackageName,
    version: VersionReq,
}

impl FromStr for PackageSpecifier {
    type Err = PackageSpecifierError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (name_s, version_s) = match value.find(|c: char| !c.is_ascii_alphanumeric()) {
            Some(idx) => value.split_at(idx),
            None => (value, "*"),
        };

        let name: PackageName = name_s.parse()?;
        let version: VersionReq = version_s.parse()?;

        Ok(PackageSpecifier { name, version })
    }
}

#[derive(Error, Debug)]
pub enum MQPkgError {
    #[error(transparent)]
    DBError(#[from] pkgdb::DBError),

    #[error(transparent)]
    RepositoryError(#[from] repository::RepositoryError),

    #[error(transparent)]
    VersionError(#[from] VersionError),
}

pub struct MQPkg {
    config: config::Config,
    db: pkgdb::Database,
}

impl MQPkg {
    pub fn new(config: config::Config, fs: VfsPath, rid: &str) -> Result<MQPkg, MQPkgError> {
        // We're using MD5 here because it's short and fast, we're not using
        // this in a security sensitive aspect.
        let id = format!("{:x}", md5::compute(rid));
        let db = pkgdb::Database::new(fs, id)?;

        Ok(MQPkg { config, db })
    }

    pub fn install(&mut self, packages: &[PackageSpecifier]) -> Result<(), MQPkgError> {
        transaction!(self.db, {
            for package in packages {
                self.db.add(package)?;
            }

            self.resolve()?;
        });

        Ok(())
    }
}

impl MQPkg {
    fn resolve(&self) -> Result<(), MQPkgError> {
        let mut repository = repository::Repository::new()?;
        repository.fetch(self.config.repositories())?;

        let solver = resolver::Solver::new(repository);
        let solution = solver.resolve(PackageName(".".to_string()), Version::parse("1.0.0")?);

        println!("{:?}", solution);

        Ok(())
    }
}
