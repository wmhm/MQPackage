// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::clone::Clone;
use std::cmp::{Eq, Ord, PartialEq};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::str::FromStr;

use pubgrub::report::DerivationTree;
use pubgrub::type_aliases::SelectedDependencies;
use semver::{Version as SemVer, VersionReq};
use serde::{Deserialize, Serialize};

use crate::errors::{PackageNameError, PackageSpecifierError, VersionError};

#[derive(Serialize, Deserialize, Clone, Eq, Debug, Hash, PartialEq)]
pub struct PackageName(String);

impl PackageName {
    pub(crate) fn new<S: Into<String>>(s: S) -> PackageName {
        PackageName(s.into())
    }
}

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

#[derive(Deserialize, Debug, Clone, Ord, Eq, PartialEq, PartialOrd, Hash)]
pub struct Version(SemVer);

impl Version {
    pub(crate) fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version(SemVer::new(major, minor, patch))
    }

    pub(crate) fn major(&self) -> u64 {
        self.0.major
    }

    pub(crate) fn minor(&self) -> u64 {
        self.0.minor
    }

    pub(crate) fn patch(&self) -> u64 {
        self.0.patch
    }
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Version(SemVer::parse(value)?))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, Debug, Hash, PartialEq)]
pub struct PackageSpecifier {
    pub(crate) name: PackageName,
    pub(crate) version: VersionReq,
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

pub(crate) type SolverSolution = SelectedDependencies<PackageName, Version>;

pub(crate) type RequestedPackages = HashMap<PackageName, VersionReq>;

pub type DerivedResult = DerivationTree<PackageName, Version>;
