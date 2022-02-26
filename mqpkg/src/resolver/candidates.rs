// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::cmp::Ordering;
use std::fmt;

use pubgrub::range::Range;
use pubgrub::version::Version as PVersion;
use pubgrub::version_set::VersionSet;
use semver::{Version, VersionReq};

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
}

impl fmt::Display for CandidateSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.range)
    }
}

impl CandidateSet {
    pub(super) fn req(req: VersionReq) -> CandidateSet {
        let mut range = Range::full();

        for comp in req.comparators.iter() {
            range = range.intersection(&convert(comp));
        }

        CandidateSet { range }
    }
}

impl VersionSet for CandidateSet {
    type V = Candidate;

    fn empty() -> CandidateSet {
        CandidateSet {
            range: Range::none(),
        }
    }

    fn singleton(v: Candidate) -> CandidateSet {
        CandidateSet {
            range: Range::exact(v),
        }
    }

    fn complement(&self) -> CandidateSet {
        CandidateSet {
            range: self.range.negate(),
        }
    }

    fn intersection(&self, other: &CandidateSet) -> CandidateSet {
        CandidateSet {
            range: self.range.intersection(&other.range),
        }
    }

    fn contains(&self, v: &Candidate) -> bool {
        self.range.contains(v)
    }
}

fn convert(comp: &semver::Comparator) -> Range<Candidate> {
    let major = comp.major;
    match comp.op {
        semver::Op::Exact => match (comp.minor, comp.patch) {
            //  =I.J.K — exactly the version I.J.K
            (Some(minor), Some(patch)) => Range::exact(Candidate::from_parts(major, minor, patch)),
            // =I.J — equivalent to >=I.J.0, <I.(J+1).0
            (Some(minor), None) => Range::between(
                Candidate::from_parts(major, minor, 0),
                Candidate::from_parts(major, minor + 1, 0),
            ),
            // =I — equivalent to >=I.0.0, <(I+1).0.0
            (None, None) => Range::between(
                Candidate::from_parts(major, 0, 0),
                Candidate::from_parts(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        semver::Op::Greater => match (comp.minor, comp.patch) {
            // >I.J.K
            (Some(minor), Some(patch)) => {
                Range::higher_than(Candidate::from_parts(major, minor, patch + 1))
            }
            // >I.J — equivalent to >=I.(J+1).0
            (Some(minor), None) => Range::higher_than(Candidate::from_parts(major, minor + 1, 0)),
            // >I — equivalent to >=(I+1).0.0
            (None, None) => Range::higher_than(Candidate::from_parts(major + 1, 0, 0)),
            _ => unreachable!(),
        },
        semver::Op::GreaterEq => match (comp.minor, comp.patch) {
            //  >=I.J.K
            (Some(minor), Some(patch)) => {
                Range::higher_than(Candidate::from_parts(major, minor, patch))
            }
            // >=I.J — equivalent to >=I.J.0
            (Some(minor), None) => Range::higher_than(Candidate::from_parts(major, minor, 0)),
            // >=I — equivalent to >=I.0.0
            (None, None) => Range::higher_than(Candidate::from_parts(major, 0, 0)),
            _ => unreachable!(),
        },
        semver::Op::Less => match (comp.minor, comp.patch) {
            // <I.J.K
            (Some(minor), Some(patch)) => {
                Range::strictly_lower_than(Candidate::from_parts(major, minor, patch))
            }
            // <I.J — equivalent to <I.J.0
            (Some(minor), None) => {
                Range::strictly_lower_than(Candidate::from_parts(major, minor, 0))
            }
            // <I — equivalent to <I.0.0
            (None, None) => Range::strictly_lower_than(Candidate::from_parts(major, 0, 0)),
            _ => unreachable!(),
        },
        semver::Op::LessEq => match (comp.minor, comp.patch) {
            // <=I.J.K — equivalent to <I.J.(K+1)
            (Some(minor), Some(patch)) => {
                Range::strictly_lower_than(Candidate::from_parts(major, minor, patch + 1))
            }
            // <=I.J — equivalent to <I.(J+1).0
            (Some(minor), None) => {
                Range::strictly_lower_than(Candidate::from_parts(major, minor + 1, 0))
            }
            // <=I — equivalent to <(I+1).0.0
            (None, None) => Range::strictly_lower_than(Candidate::from_parts(major + 1, 0, 0)),
            _ => unreachable!(),
        },
        semver::Op::Tilde => match (comp.minor, comp.patch) {
            // ~I.J.K — equivalent to >=I.J.K, <I.(J+1).0
            (Some(minor), Some(patch)) => Range::between(
                Candidate::from_parts(major, minor, patch),
                Candidate::from_parts(major, minor + 1, 0),
            ),
            // ~I.J — equivalent to =I.J
            (Some(minor), None) => Range::between(
                Candidate::from_parts(major, minor, 0),
                Candidate::from_parts(major, minor + 1, 0),
            ),
            // ~I — equivalent to =I
            (None, None) => Range::between(
                Candidate::from_parts(major, 0, 0),
                Candidate::from_parts(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        semver::Op::Caret => match (comp.minor, comp.patch) {
            (Some(minor), Some(patch)) => {
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
            (Some(minor), None) => {
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
            (None, None) => Range::between(
                Candidate::from_parts(major, 0, 0),
                Candidate::from_parts(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        semver::Op::Wildcard => match (comp.minor, comp.patch) {
            (Some(_), Some(_)) => unreachable!(),
            (Some(minor), None) => Range::between(
                Candidate::from_parts(major, minor, 0),
                Candidate::from_parts(major, minor + 1, 0),
            ),
            (None, None) => Range::between(
                Candidate::from_parts(major, 0, 0),
                Candidate::from_parts(major + 1, 0, 0),
            ),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
