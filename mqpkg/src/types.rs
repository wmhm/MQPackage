// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::clone::Clone;
use std::cmp::Ordering;
use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::str::FromStr;

use semver::{Prerelease, Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::errors::{PackageNameError, PackageSpecifierError};

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

pub(crate) type RequestedPackages = HashMap<PackageName, VersionReq>;

#[derive(Debug, Clone)]
pub struct Candidate {
    is_root: bool,
    version: Version,
}

impl Candidate {
    pub(crate) fn version(&self) -> &Version {
        &self.version
    }
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}
impl Eq for Candidate {}

impl Candidate {
    pub(crate) fn new(version: Version) -> Candidate {
        Candidate {
            is_root: false,
            version,
        }
    }

    pub(crate) fn root() -> Candidate {
        Candidate {
            is_root: true,
            version: Version::new(0, 0, 0),
        }
    }

    pub(crate) fn from_parts(major: u64, minor: u64, patch: u64) -> Candidate {
        Candidate {
            is_root: false,
            version: Version::new(major, minor, patch),
        }
    }

    pub(crate) fn from_parts_pre(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Candidate {
        let mut version = Version::new(major, minor, patch);
        version.pre = pre;

        Candidate {
            is_root: false,
            version,
        }
    }
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Note: This relies on a hack where our root versions have an internal,
        // otherwise ignored, marker that suppresses their fmt::Display output.
        //
        // This would be better handled by a custom reporter, but that can be
        // done later.
        if !self.is_root {
            write!(f, "{}", self.version)?
        }

        Ok(())
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Candidate) -> Ordering {
        self.version.cmp(&other.version)
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Candidate) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
