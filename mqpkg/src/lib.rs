// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::clone::Clone;
use std::cmp::{Eq, PartialEq};
use std::hash::Hash;
use std::str::FromStr;

use semver::VersionReq;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vfs::VfsPath;

pub mod config;

mod pkgdb;

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
}

pub struct MQPkg {
    db: pkgdb::Database,
}

impl MQPkg {
    pub fn new(_config: config::Config, fs: VfsPath) -> Result<MQPkg, MQPkgError> {
        let db = pkgdb::Database::new(fs)?;

        Ok(MQPkg { db })
    }

    pub fn install(&mut self, packages: &[PackageSpecifier]) -> Result<(), MQPkgError> {
        for package in packages {
            self.db.add(package)?;
        }

        Ok(())
    }
}
