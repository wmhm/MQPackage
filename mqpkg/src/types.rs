// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::clone::Clone;
use std::cmp::{Eq, PartialEq};
use std::collections::BTreeMap;
use std::fmt;
use std::hash::Hash;
use std::str::FromStr;

use dyn_clone::DynClone;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::errors::{PackageNameError, PackageSpecifierError};

#[derive(Serialize, Deserialize, Clone, Eq, Debug, Hash, PartialEq, Ord, PartialOrd)]
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

pub(crate) type Packages = BTreeMap<PackageName, Package>;

pub(crate) trait Source: fmt::Debug + fmt::Display + DynClone + Sync + Send {
    fn id(&self) -> u64;

    fn discriminator(&self) -> u64;
}

dyn_clone::clone_trait_object!(Source);

pub(crate) trait WithSource {
    #[allow(clippy::borrowed_box)]
    fn source(&self) -> &Box<dyn Source>;
}

pub(crate) struct Package {
    name: PackageName,
    version: Version,
    source: Box<dyn Source>,
}

impl Package {
    pub(crate) fn new<P: Into<PackageName>, V: Into<Version>>(
        name: P,
        version: V,
        source: Box<dyn Source>,
    ) -> Package {
        Package {
            name: name.into(),
            version: version.into(),
            source,
        }
    }
}

impl WithSource for Package {
    fn source(&self) -> &Box<dyn Source> {
        &self.source
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} at {} from {}",
            self.name,
            self.version,
            self.source()
        )
    }
}
