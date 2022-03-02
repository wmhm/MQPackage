// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use crate::resolver::pubgrub::versionset::Candidate as CandidateTrait;
use crate::resolver::types::{
    Dependencies, Name, Requirement, StaticDependencies, Version, WithDependencies,
};
use crate::types::{Source, WithSource};

#[derive(Debug, Clone)]
struct InternalSource(u64);

impl InternalSource {
    fn new(id: u64) -> InternalSource {
        InternalSource(id)
    }
}

impl fmt::Display for InternalSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Internal")
    }
}

impl Source for InternalSource {
    fn id(&self) -> u64 {
        0
    }

    fn discriminator(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Candidate {
    version: Version,
    source: Box<dyn Source>,
    dependencies: Box<dyn Dependencies + Sync + Send>,
}

impl Candidate {
    pub(crate) fn new<V: Into<Version>>(
        version: V,
        source: Box<dyn Source>,
        dependencies: Box<dyn Dependencies + Sync + Send>,
    ) -> Candidate {
        Candidate {
            version: version
                .into()
                .with_source_id(source.id())
                .with_source_discriminator(source.discriminator()),
            source,
            dependencies,
        }
    }

    pub(in crate::resolver) fn root<N: Into<Name>, R: Into<Requirement>>(
        reqs: HashMap<N, R>,
    ) -> Candidate {
        Candidate {
            version: Version::candidate(0, 0, 0),
            source: Box::new(InternalSource::new(0)),
            dependencies: Box::new(StaticDependencies::new(
                reqs.into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            )),
        }
    }
}

impl WithDependencies for Candidate {
    fn dependencies(&self) -> &dyn Dependencies {
        &*self.dependencies
    }
}

impl WithSource for Candidate {
    fn source(&self) -> &Box<dyn Source> {
        &self.source
    }
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.version)
    }
}

impl PartialEq for Candidate {
    // It's important that this is just dispatched to Version, otherwise the
    // internal Range will get confused.
    fn eq(&self, other: &Candidate) -> bool {
        self.version == other.version
    }
}

impl Eq for Candidate {}

impl Ord for Candidate {
    // It's important that this is just dispatched to Version, otherwise the
    // internal Range will get confused.
    fn cmp(&self, other: &Candidate) -> Ordering {
        self.version.cmp(&other.version)
    }
}

impl PartialOrd for Candidate {
    // It's important that this is just dispatched to Version, otherwise the
    // internal Range will get confused.
    fn partial_cmp(&self, other: &Candidate) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl CandidateTrait for Candidate {
    type V = Version;

    fn version(&self) -> &Version {
        &self.version
    }
}
