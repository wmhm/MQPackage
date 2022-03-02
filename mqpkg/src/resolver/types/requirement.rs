// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::fmt;

use pubgrub::version_set::VersionSet as BaseVersionSet;
use semver::{Prerelease, VersionReq};

use crate::resolver::pubgrub::{Candidate, VersionSet};
use crate::resolver::types::version::Version;

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

impl From<VersionReq> for Requirement {
    fn from(req: VersionReq) -> Requirement {
        Requirement::new(req)
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
