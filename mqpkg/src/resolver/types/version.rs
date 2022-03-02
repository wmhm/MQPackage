// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::cmp::Ordering;
use std::fmt;

use pubgrub::version::Version as PubGrubVersion;

use crate::resolver::pubgrub::CandidateVersion;

#[derive(Debug, Clone)]
pub struct Version {
    version: semver::Version,
    source_id: u64,
    source_discriminator: u64,
    suppress_display: bool,
}

impl Version {
    fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            version: semver::Version::new(major, minor, patch),
            source_id: 0,
            source_discriminator: 0,
            suppress_display: false,
        }
    }

    pub(in crate::resolver) fn candidate(major: u64, minor: u64, patch: u64) -> Version {
        Version::new(major, minor, patch).with_source_id(u64::MAX)
    }

    pub(in crate::resolver) fn pre<S: AsRef<str>>(mut self, pre: S) -> Version {
        self.version.pre = semver::Prerelease::new(pre.as_ref()).unwrap();
        self
    }

    pub(in crate::resolver) fn with_source_id(mut self, source_id: u64) -> Version {
        self.source_id = source_id;
        self
    }

    pub(in crate::resolver) fn with_source_discriminator(
        mut self,
        source_discriminator: u64,
    ) -> Version {
        self.source_discriminator = source_discriminator;
        self
    }

    pub(in crate::resolver) fn suppress_display(mut self) -> Version {
        self.suppress_display = true;
        self
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.suppress_display {
            write!(f, "{}", self.version)?;
        }

        Ok(())
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        (&self.version, self.source_id, self.source_discriminator)
            == (&other.version, other.source_id, other.source_discriminator)
    }
}
impl Eq for Version {}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.version.cmp(&other.version) {
            Ordering::Equal => (self.source_id, self.source_discriminator)
                .cmp(&(other.source_id, other.source_discriminator))
                .reverse(),
            Ordering::Greater => Ordering::Greater,
            Ordering::Less => Ordering::Less,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl CandidateVersion for Version {
    fn is_prerelease(&self) -> bool {
        !self.version.pre.is_empty()
    }
}

impl PubGrubVersion for Version {
    fn lowest() -> Version {
        Version::candidate(0, 0, 0)
    }

    fn bump(&self) -> Version {
        Version::new(
            self.version.major,
            self.version.minor,
            self.version.patch + 1,
        )
        .with_source_id(self.source_id)
    }
}

impl From<&semver::Version> for Version {
    fn from(version: &semver::Version) -> Version {
        Version {
            version: version.clone(),
            source_id: 0,
            source_discriminator: 0,
            suppress_display: false,
        }
    }
}

impl From<&Version> for semver::Version {
    fn from(version: &Version) -> semver::Version {
        version.version.clone()
    }
}
