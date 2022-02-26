// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::cmp::Ordering;
use std::fmt;

use pubgrub::range::Range;
use pubgrub::version::Version as PVersion;
use pubgrub::version_set::VersionSet;
use semver::{Prerelease, Version, VersionReq};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Candidate {
    pub(super) version: Version,
}

impl Candidate {
    pub(super) fn new(version: Version) -> Candidate {
        Candidate { version }
    }

    fn from_parts(major: u64, minor: u64, patch: u64) -> Candidate {
        Candidate {
            version: Version::new(major, minor, patch),
        }
    }

    fn from_parts_pre(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Candidate {
        let mut version = Version::new(major, minor, patch);
        version.pre = pre;

        Candidate { version }
    }
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.version)
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

impl PVersion for Candidate {
    fn lowest() -> Candidate {
        Candidate::new(Version::new(0, 0, 0))
    }

    fn bump(&self) -> Candidate {
        Candidate::new(Version::new(
            self.version.major,
            self.version.minor,
            self.version.patch + 1,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateSet {
    range: Range<Candidate>,
    pre: Range<Candidate>,
}

impl fmt::Display for CandidateSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.range)
    }
}

impl CandidateSet {
    pub(super) fn req(req: VersionReq) -> CandidateSet {
        // By default, we allow *any* normal version to be accepted,
        // then we futher constrain those down.
        let mut range = Range::full();
        // By default, we allow *no* pre-release versions to be accepted,
        // then we start allowing additional pre-releases via Unions.
        let mut pre = Range::none();

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
        for comp in req.comparators.iter() {
            range = range.intersection(&convert_normal(comp));
            pre = pre.union(&convert_prerelease(comp));
        }

        // Since we're going to use only pre when checking against
        // a pre-release, we still need to compute the intersection
        // of what we would normally allow, against our union of
        // of explicitly mentioned pre-releases.
        pre = range.intersection(&pre);

        CandidateSet { range, pre }
    }
}

impl VersionSet for CandidateSet {
    type V = Candidate;

    fn empty() -> CandidateSet {
        CandidateSet {
            range: Range::none(),
            pre: Range::none(),
        }
    }

    fn singleton(v: Candidate) -> CandidateSet {
        // This relies on the fact that we have range, for when
        // our incoming version is a "normal" version number, and
        // pre for when it's a pre-release.
        //
        // When our exact version is a final release, if our incoming
        // version is a pre-releaase, then it can't match (thus range
        // is None), and if it isn't a pre-release then normal
        // Range::exact will function.
        //
        // When our exact version is a pre-release, if our incoming
        // version is not, then it can't match (thus range is None)
        // and if it is, then normal Range::exact() can match.
        if v.version.pre.is_empty() {
            CandidateSet {
                range: Range::exact(v),
                pre: Range::none(),
            }
        } else {
            CandidateSet {
                range: Range::none(),
                pre: Range::exact(v),
            }
        }
    }

    fn complement(&self) -> CandidateSet {
        CandidateSet {
            range: self.range.negate(),
            pre: self.pre.negate(),
        }
    }

    fn intersection(&self, other: &CandidateSet) -> CandidateSet {
        CandidateSet {
            range: self.range.intersection(&other.range),
            pre: self.pre.intersection(&other.pre),
        }
    }

    fn contains(&self, v: &Candidate) -> bool {
        // We use different logic here depending on if the version
        // we're checking against is a pre-release or not. If it is
        // a pre-release, then we check it againt our prerelease
        // range. Otherwise our standard range is accurate.
        //
        // This is done because our standard range would technically
        // allow any pre-releases, even ones for many versions later.
        // So we have a pre-release range, that matches all of the
        // normal stuff, but also ensures that any pre-release version
        // we accept, had been explicitly mentioned. So that something
        // like >=I.J.K-P would allow all pre-releases for I.J.K, but
        // would not then allow another one for I.K.(K+1)-P.
        //
        // So in this way, when checking if a set contains a pre-release
        // we essentially have an additional constraint of >=I.J.K-P, <I.J.K,
        // for each explicitly mentioned pre-release.
        //
        // However, that same thing does not hold true when checking a
        // final release, as we have no need to additionally constrain
        // those.
        if v.version.pre.is_empty() {
            self.range.contains(v)
        } else {
            self.pre.contains(v)
        }
    }
}

fn bump_pre(pre: Prerelease) -> Prerelease {
    Prerelease::new(&(pre.to_string() + ".0")).unwrap()
}

fn convert_prerelease(comp: &semver::Comparator) -> Range<Candidate> {
    if comp.pre.is_empty() {
        Range::none()
    } else {
        Range::between(
            Candidate::from_parts_pre(
                comp.major,
                comp.minor.unwrap(),
                comp.patch.unwrap(),
                comp.pre.clone(),
            ),
            Candidate::from_parts(comp.major, comp.minor.unwrap(), comp.patch.unwrap()),
        )
    }
}

fn convert_normal(comp: &semver::Comparator) -> Range<Candidate> {
    let major = comp.major;
    let comp_pre = if comp.pre.is_empty() {
        None
    } else {
        Some(comp.pre.clone())
    };

    match comp.op {
        semver::Op::Exact => match (comp.minor, comp.patch, comp_pre) {
            //  =I.J.K-P — equivalent to >=I.J.K-P, <I.J.K
            (Some(minor), Some(patch), Some(pre)) => Range::between(
                Candidate::from_parts_pre(major, minor, patch, pre),
                Candidate::from_parts(major, minor, patch),
            ),
            //  =I.J.K — exactly the version I.J.K
            (Some(minor), Some(patch), None) => {
                Range::exact(Candidate::from_parts(major, minor, patch))
            }
            // =I.J — equivalent to >=I.J.0, <I.(J+1).0
            (Some(minor), None, None) => Range::between(
                Candidate::from_parts(major, minor, 0),
                Candidate::from_parts(major, minor + 1, 0),
            ),
            // =I — equivalent to >=I.0.0, <(I+1).0.0
            (None, None, None) => Range::between(
                Candidate::from_parts(major, 0, 0),
                Candidate::from_parts(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        semver::Op::Greater => {
            match (comp.minor, comp.patch, comp_pre) {
                // >I.J.K-P
                (Some(minor), Some(patch), Some(pre)) => Range::higher_than(
                    Candidate::from_parts_pre(major, minor, patch, bump_pre(pre)),
                ),
                // >I.J.K
                (Some(minor), Some(patch), None) => {
                    Range::higher_than(Candidate::from_parts(major, minor, patch + 1))
                }
                // >I.J — equivalent to >=I.(J+1).0
                (Some(minor), None, None) => {
                    Range::higher_than(Candidate::from_parts(major, minor + 1, 0))
                }
                // >I — equivalent to >=(I+1).0.0
                (None, None, None) => Range::higher_than(Candidate::from_parts(major + 1, 0, 0)),
                _ => unreachable!(),
            }
        }
        semver::Op::GreaterEq => match (comp.minor, comp.patch, comp_pre) {
            //  >=I.J.K-P
            (Some(minor), Some(patch), Some(pre)) => {
                Range::higher_than(Candidate::from_parts_pre(major, minor, patch, pre))
            }
            //  >=I.J.K
            (Some(minor), Some(patch), None) => {
                Range::higher_than(Candidate::from_parts(major, minor, patch))
            }
            // >=I.J — equivalent to >=I.J.0
            (Some(minor), None, None) => Range::higher_than(Candidate::from_parts(major, minor, 0)),
            // >=I — equivalent to >=I.0.0
            (None, None, None) => Range::higher_than(Candidate::from_parts(major, 0, 0)),
            _ => unreachable!(),
        },
        semver::Op::Less => match (comp.minor, comp.patch, comp_pre) {
            // <I.J.K-P
            (Some(minor), Some(patch), Some(pre)) => {
                Range::strictly_lower_than(Candidate::from_parts_pre(major, minor, patch, pre))
            }
            // <I.J.K
            (Some(minor), Some(patch), None) => {
                Range::strictly_lower_than(Candidate::from_parts(major, minor, patch))
            }
            // <I.J — equivalent to <I.J.0
            (Some(minor), None, None) => {
                Range::strictly_lower_than(Candidate::from_parts(major, minor, 0))
            }
            // <I — equivalent to <I.0.0
            (None, None, None) => Range::strictly_lower_than(Candidate::from_parts(major, 0, 0)),
            _ => unreachable!(),
        },
        semver::Op::LessEq => {
            match (comp.minor, comp.patch, comp_pre) {
                // <=I.J.K-P — equivalent to <I.J.K-(P.0)
                (Some(minor), Some(patch), Some(pre)) => Range::strictly_lower_than(
                    Candidate::from_parts_pre(major, minor, patch, bump_pre(pre)),
                ),
                // <=I.J.K — equivalent to <I.J.(K+1)
                (Some(minor), Some(patch), None) => {
                    Range::strictly_lower_than(Candidate::from_parts(major, minor, patch + 1))
                }
                // <=I.J — equivalent to <I.(J+1).0
                (Some(minor), None, None) => {
                    Range::strictly_lower_than(Candidate::from_parts(major, minor + 1, 0))
                }
                // <=I — equivalent to <(I+1).0.0
                (None, None, None) => {
                    Range::strictly_lower_than(Candidate::from_parts(major + 1, 0, 0))
                }
                _ => unreachable!(),
            }
        }
        semver::Op::Tilde => match (comp.minor, comp.patch, comp_pre) {
            // ~I.J.K — equivalent to >=I.J.K-P, <I.(J+1).0
            (Some(minor), Some(patch), Some(pre)) => Range::between(
                Candidate::from_parts_pre(major, minor, patch, pre),
                Candidate::from_parts(major, minor + 1, 0),
            ),
            // ~I.J.K — equivalent to >=I.J.K, <I.(J+1).0
            (Some(minor), Some(patch), None) => Range::between(
                Candidate::from_parts(major, minor, patch),
                Candidate::from_parts(major, minor + 1, 0),
            ),
            // ~I.J — equivalent to =I.J
            (Some(minor), None, None) => Range::between(
                Candidate::from_parts(major, minor, 0),
                Candidate::from_parts(major, minor + 1, 0),
            ),
            // ~I — equivalent to =I
            (None, None, None) => Range::between(
                Candidate::from_parts(major, 0, 0),
                Candidate::from_parts(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        semver::Op::Caret => match (comp.minor, comp.patch, comp_pre) {
            (Some(minor), Some(patch), Some(pre)) => {
                if major > 0 {
                    // ^I.J.K-P (for I>0) — equivalent to >=I.J.K-P, <(I+1).0.0
                    Range::between(
                        Candidate::from_parts_pre(major, minor, patch, pre),
                        Candidate::from_parts(major + 1, 0, 0),
                    )
                } else if minor > 0 {
                    // ^0.J.K (for J>0) — equivalent to >=0.J.K-P, <0.(J+1).0
                    assert!(major == 0);
                    Range::between(
                        Candidate::from_parts_pre(0, minor, patch, pre),
                        Candidate::from_parts(0, minor + 1, 0),
                    )
                } else {
                    // ^0.0.K-P — equivalent to  >=I.J.K-P, <I.J.K
                    assert!(major == 0 && minor == 0);
                    Range::between(
                        Candidate::from_parts_pre(major, minor, patch, pre),
                        Candidate::from_parts(major, minor, patch),
                    )
                }
            }
            (Some(minor), Some(patch), None) => {
                if major > 0 {
                    // ^I.J.K (for I>0) — equivalent to >=I.J.K, <(I+1).0.0
                    Range::between(
                        Candidate::from_parts(major, minor, patch),
                        Candidate::from_parts(major + 1, 0, 0),
                    )
                } else if minor > 0 {
                    // ^0.J.K (for J>0) — equivalent to >=0.J.K, <0.(J+1).0
                    assert!(major == 0);
                    Range::between(
                        Candidate::from_parts(0, minor, patch),
                        Candidate::from_parts(0, minor + 1, 0),
                    )
                } else {
                    // ^0.0.K — equivalent to =0.0.K
                    assert!(major == 0 && minor == 0);
                    Range::exact(Candidate::from_parts(0, 0, patch))
                }
            }
            (Some(minor), None, None) => {
                if major > 0 || minor > 0 {
                    // ^I.J (for I>0 or J>0) — equivalent to ^I.J.0
                    Range::between(
                        Candidate::from_parts(major, minor, 0),
                        Candidate::from_parts(major + 1, 0, 0),
                    )
                } else {
                    // ^0.0 — equivalent to =0.0
                    assert!(major == 0 && minor == 0);
                    Range::between(
                        Candidate::from_parts(major, minor, 0),
                        Candidate::from_parts(major, minor + 1, 0),
                    )
                }
            }
            // ^I — equivalent to =I
            (None, None, None) => Range::between(
                Candidate::from_parts(major, 0, 0),
                Candidate::from_parts(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        semver::Op::Wildcard => match (comp.minor, comp.patch) {
            (Some(_), Some(_)) => unreachable!(),
            // I.J.* — equivalent to =I.J
            (Some(minor), None) => Range::between(
                Candidate::from_parts(major, minor, 0),
                Candidate::from_parts(major, minor + 1, 0),
            ),
            // I.* or I.*.* — equivalent to =I
            (None, None) => Range::between(
                Candidate::from_parts(major, 0, 0),
                Candidate::from_parts(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
