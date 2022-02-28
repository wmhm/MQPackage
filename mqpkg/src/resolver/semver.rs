// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use ::pubgrub::version::Version as PubGrubVersion;
use ::pubgrub::version_set::VersionSet as PubGrubVersionSet;
use dyn_clone::DynClone;
use semver::{Prerelease, Version as SemVer, VersionReq};

use crate::resolver::pubgrub;
use crate::types::{PackageName, RequestedPackages};

pub(super) use super::pubgrub::VersionSet;

pub(crate) trait Dependencies: fmt::Debug + DynClone {
    fn get(&self) -> HashMap<PackageName, Requirement>;
}

dyn_clone::clone_trait_object!(Dependencies);

#[derive(Debug, Clone)]
struct StaticDependencies {
    dependencies: HashMap<PackageName, Requirement>,
}

impl StaticDependencies {
    fn new(dependencies: HashMap<PackageName, Requirement>) -> StaticDependencies {
        StaticDependencies { dependencies }
    }
}

impl Dependencies for StaticDependencies {
    fn get(&self) -> HashMap<PackageName, Requirement> {
        self.dependencies.clone()
    }
}

pub(crate) trait WithDependencies {
    fn dependencies(&self) -> &dyn Dependencies;
}

pub(crate) trait Source: fmt::Debug + fmt::Display + DynClone {
    fn id(&self) -> u64;

    fn discriminator(&self) -> u64;
}

dyn_clone::clone_trait_object!(Source);

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

pub(super) trait WithSource {
    fn source(&self) -> &dyn Source;
}

#[derive(Debug, Clone)]
pub struct Version {
    version: SemVer,
    source_id: u64,
    source_discriminator: u64,
}

impl Version {
    fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            version: SemVer::new(major, minor, patch),
            source_id: 0,
            source_discriminator: 0,
        }
    }

    fn candidate(major: u64, minor: u64, patch: u64) -> Version {
        Version::new(major, minor, patch).with_source_id(u64::MAX)
    }

    fn pre<S: AsRef<str>>(mut self, pre: S) -> Version {
        self.version.pre = Prerelease::new(pre.as_ref()).unwrap();
        self
    }

    fn with_source_id(mut self, source_id: u64) -> Version {
        self.source_id = source_id;
        self
    }

    fn with_source_discriminator(mut self, source_discriminator: u64) -> Version {
        self.source_discriminator = source_discriminator;
        self
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.version)
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

impl pubgrub::Version for Version {
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

impl From<&SemVer> for Version {
    fn from(version: &SemVer) -> Version {
        Version {
            version: version.clone(),
            source_id: 0,
            source_discriminator: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Requirement(VersionReq);

impl Requirement {
    pub(crate) fn new(req: VersionReq) -> Requirement {
        Requirement(req)
    }
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&VersionReq> for Requirement {
    fn from(req: &VersionReq) -> Requirement {
        Requirement::new(req.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Candidate {
    version: Version,
    source: Box<dyn Source + Sync + Send>,
    dependencies: Box<dyn Dependencies + Sync + Send>,
}

impl Candidate {
    pub(crate) fn new<V: Into<Version>>(
        version: V,
        source: Box<dyn Source + Sync + Send>,
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

    pub(super) fn root(reqs: RequestedPackages) -> Candidate {
        Candidate {
            version: Version::candidate(0, 0, 0),
            source: Box::new(InternalSource::new(0)),
            dependencies: Box::new(StaticDependencies::new(
                reqs.iter()
                    .map(|(k, v)| (k.clone(), Requirement(v.clone())))
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
    fn source(&self) -> &dyn Source {
        &*self.source
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

impl pubgrub::Candidate for Candidate {
    type V = Version;

    fn version(&self) -> &Version {
        &self.version
    }
}

impl From<&Requirement> for VersionSet<Candidate> {
    fn from(req: &Requirement) -> VersionSet<Candidate> {
        // By default, we allow *any* normal version to be accepted,
        // then we futher constrain those down.
        // let mut range = Range::full();
        // By default, we allow *no* pre-release versions to be accepted,
        // then we start allowing additional pre-releases via Unions.
        // let mut pre = Range::none();
        let mut vs = VersionSet::default();

        // This whole thing is subtle, but our "range" here will
        // only be used when we're trying to see if a non pre-release
        // (aka a "final") version is contained within this set. So
        // for that, we just compute the normal intersection of all
        // requirements.
        //
        // However, for "pre", which is used when we're trying to see
        // if a pre-release is contained within this set, we still need
        // to apply all of the same logic of an intersection of all
        // of the requirements. On top of that, we don't want to use
        // a pre-release version unless a requirement has *explicitly*
        // mentioned it, though we will accept later pre-releases for
        // the same version.
        //
        // Thus, pre-releases effectively have an additional constraint,
        // which is a union of all pre-release versions mentioned
        // constrained so: >=I.J.K-P, <I.J.(K+1). This ensures that a
        // pre-release version had to have been explicitly mentioned
        // (or is a direct upgrade to it).
        for comp in req.0.comparators.iter() {
            vs = vs.with_normal(&convert_normal(comp));
            vs = vs.with_pre(&convert_prerelease(comp));
        }

        // Since we're going to use only pre when checking against
        // a pre-release, we still need to compute the intersection
        // of what we would normally allow, against our union of
        // of explicitly mentioned pre-releases.
        // pre = range.intersection(&pre);

        vs
    }
}

fn bump_pre<S: AsRef<str>>(pre: S) -> String {
    let new_str = format!("{}.0", pre.as_ref());
    Prerelease::new(new_str.as_ref()).unwrap().to_string()
}

fn convert_prerelease(comp: &semver::Comparator) -> VersionSet<Candidate> {
    if comp.pre.is_empty() {
        VersionSet::empty()
    } else {
        VersionSet::between(
            Version::candidate(comp.major, comp.minor.unwrap(), comp.patch.unwrap())
                .pre(comp.pre.as_str()),
            Version::candidate(comp.major, comp.minor.unwrap(), comp.patch.unwrap()),
        )
    }
}

fn convert_normal(comp: &semver::Comparator) -> VersionSet<Candidate> {
    let major = comp.major;
    let comp_pre = if comp.pre.is_empty() {
        None
    } else {
        Some(comp.pre.as_str())
    };

    match comp.op {
        semver::Op::Exact => match (comp.minor, comp.patch, comp_pre) {
            //  =I.J.K-P — equivalent to >=I.J.K-P, <I.J.K
            (Some(minor), Some(patch), Some(pre)) => VersionSet::between(
                Version::candidate(major, minor, patch).pre(pre),
                Version::candidate(major, minor, patch),
            ),
            //  =I.J.K — exactly the version I.J.K
            (Some(minor), Some(patch), None) => {
                VersionSet::exact(Version::candidate(major, minor, patch))
            }
            // =I.J — equivalent to >=I.J.0, <I.(J+1).0
            (Some(minor), None, None) => VersionSet::between(
                Version::candidate(major, minor, 0),
                Version::candidate(major, minor + 1, 0),
            ),
            // =I — equivalent to >=I.0.0, <(I+1).0.0
            (None, None, None) => VersionSet::between(
                Version::candidate(major, 0, 0),
                Version::candidate(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        semver::Op::Greater => {
            match (comp.minor, comp.patch, comp_pre) {
                // >I.J.K-P
                (Some(minor), Some(patch), Some(pre)) => VersionSet::higher_than(
                    Version::candidate(major, minor, patch).pre(bump_pre(pre)),
                ),
                // >I.J.K
                (Some(minor), Some(patch), None) => {
                    VersionSet::higher_than(Version::candidate(major, minor, patch + 1))
                }
                // >I.J — equivalent to >=I.(J+1).0
                (Some(minor), None, None) => {
                    VersionSet::higher_than(Version::candidate(major, minor + 1, 0))
                }
                // >I — equivalent to >=(I+1).0.0
                (None, None, None) => VersionSet::higher_than(Version::candidate(major + 1, 0, 0)),
                _ => unreachable!(),
            }
        }
        semver::Op::GreaterEq => match (comp.minor, comp.patch, comp_pre) {
            //  >=I.J.K-P
            (Some(minor), Some(patch), Some(pre)) => {
                VersionSet::higher_than(Version::candidate(major, minor, patch).pre(pre))
            }
            //  >=I.J.K
            (Some(minor), Some(patch), None) => {
                VersionSet::higher_than(Version::candidate(major, minor, patch))
            }
            // >=I.J — equivalent to >=I.J.0
            (Some(minor), None, None) => {
                VersionSet::higher_than(Version::candidate(major, minor, 0))
            }
            // >=I — equivalent to >=I.0.0
            (None, None, None) => VersionSet::higher_than(Version::candidate(major, 0, 0)),
            _ => unreachable!(),
        },
        semver::Op::Less => match (comp.minor, comp.patch, comp_pre) {
            // <I.J.K-P
            (Some(minor), Some(patch), Some(pre)) => {
                VersionSet::strictly_lower_than(Version::candidate(major, minor, patch).pre(pre))
            }
            // <I.J.K
            (Some(minor), Some(patch), None) => {
                VersionSet::strictly_lower_than(Version::candidate(major, minor, patch))
            }
            // <I.J — equivalent to <I.J.0
            (Some(minor), None, None) => {
                VersionSet::strictly_lower_than(Version::candidate(major, minor, 0))
            }
            // <I — equivalent to <I.0.0
            (None, None, None) => VersionSet::strictly_lower_than(Version::candidate(major, 0, 0)),
            _ => unreachable!(),
        },
        semver::Op::LessEq => {
            match (comp.minor, comp.patch, comp_pre) {
                // <=I.J.K-P — equivalent to <I.J.K-(P.0)
                (Some(minor), Some(patch), Some(pre)) => VersionSet::strictly_lower_than(
                    Version::candidate(major, minor, patch).pre(bump_pre(pre)),
                ),
                // <=I.J.K — equivalent to <I.J.(K+1)
                (Some(minor), Some(patch), None) => {
                    VersionSet::strictly_lower_than(Version::candidate(major, minor, patch + 1))
                }
                // <=I.J — equivalent to <I.(J+1).0
                (Some(minor), None, None) => {
                    VersionSet::strictly_lower_than(Version::candidate(major, minor + 1, 0))
                }
                // <=I — equivalent to <(I+1).0.0
                (None, None, None) => {
                    VersionSet::strictly_lower_than(Version::candidate(major + 1, 0, 0))
                }
                _ => unreachable!(),
            }
        }
        semver::Op::Tilde => match (comp.minor, comp.patch, comp_pre) {
            // ~I.J.K — equivalent to >=I.J.K-P, <I.(J+1).0
            (Some(minor), Some(patch), Some(pre)) => VersionSet::between(
                Version::candidate(major, minor, patch).pre(pre),
                Version::candidate(major, minor + 1, 0),
            ),
            // ~I.J.K — equivalent to >=I.J.K, <I.(J+1).0
            (Some(minor), Some(patch), None) => VersionSet::between(
                Version::candidate(major, minor, patch),
                Version::candidate(major, minor + 1, 0),
            ),
            // ~I.J — equivalent to =I.J
            (Some(minor), None, None) => VersionSet::between(
                Version::candidate(major, minor, 0),
                Version::candidate(major, minor + 1, 0),
            ),
            // ~I — equivalent to =I
            (None, None, None) => VersionSet::between(
                Version::candidate(major, 0, 0),
                Version::candidate(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        semver::Op::Caret => match (comp.minor, comp.patch, comp_pre) {
            (Some(minor), Some(patch), Some(pre)) => {
                if major > 0 {
                    // ^I.J.K-P (for I>0) — equivalent to >=I.J.K-P, <(I+1).0.0
                    VersionSet::between(
                        Version::candidate(major, minor, patch).pre(pre),
                        Version::candidate(major + 1, 0, 0),
                    )
                } else if minor > 0 {
                    // ^0.J.K (for J>0) — equivalent to >=0.J.K-P, <0.(J+1).0
                    assert!(major == 0);
                    VersionSet::between(
                        Version::candidate(0, minor, patch).pre(pre),
                        Version::candidate(0, minor + 1, 0),
                    )
                } else {
                    // ^0.0.K-P — equivalent to  >=I.J.K-P, <I.J.K
                    assert!(major == 0 && minor == 0);
                    VersionSet::between(
                        Version::candidate(major, minor, patch).pre(pre),
                        Version::candidate(major, minor, patch),
                    )
                }
            }
            (Some(minor), Some(patch), None) => {
                if major > 0 {
                    // ^I.J.K (for I>0) — equivalent to >=I.J.K, <(I+1).0.0
                    VersionSet::between(
                        Version::candidate(major, minor, patch),
                        Version::candidate(major + 1, 0, 0),
                    )
                } else if minor > 0 {
                    // ^0.J.K (for J>0) — equivalent to >=0.J.K, <0.(J+1).0
                    assert!(major == 0);
                    VersionSet::between(
                        Version::candidate(0, minor, patch),
                        Version::candidate(0, minor + 1, 0),
                    )
                } else {
                    // ^0.0.K — equivalent to =0.0.K
                    assert!(major == 0 && minor == 0);
                    VersionSet::exact(Version::candidate(0, 0, patch))
                }
            }
            (Some(minor), None, None) => {
                if major > 0 || minor > 0 {
                    // ^I.J (for I>0 or J>0) — equivalent to ^I.J.0
                    VersionSet::between(
                        Version::candidate(major, minor, 0),
                        Version::candidate(major + 1, 0, 0),
                    )
                } else {
                    // ^0.0 — equivalent to =0.0
                    assert!(major == 0 && minor == 0);
                    VersionSet::between(
                        Version::candidate(major, minor, 0),
                        Version::candidate(major, minor + 1, 0),
                    )
                }
            }
            // ^I — equivalent to =I
            (None, None, None) => VersionSet::between(
                Version::candidate(major, 0, 0),
                Version::candidate(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        semver::Op::Wildcard => match (comp.minor, comp.patch) {
            (Some(_), Some(_)) => unreachable!(),
            // I.J.* — equivalent to =I.J
            (Some(minor), None) => VersionSet::between(
                Version::candidate(major, minor, 0),
                Version::candidate(major, minor + 1, 0),
            ),
            // I.* or I.*.* — equivalent to =I
            (None, None) => VersionSet::between(
                Version::candidate(major, 0, 0),
                Version::candidate(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
