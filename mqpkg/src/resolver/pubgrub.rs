// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::fmt;

use ::pubgrub::{
    range::Range, version::Version as PubGrubVersion, version_set::VersionSet as PubGrubVersionSet,
};

pub trait Version: PubGrubVersion {
    fn is_prerelease(&self) -> bool;
}

pub trait Candidate: fmt::Debug + fmt::Display + Clone + Eq + Ord {
    type V: Version;

    fn version(&self) -> &Self::V;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VersionSet<C: Candidate> {
    range: Range<C::V>,
    pre: Range<C::V>,
}

impl<C: Candidate> fmt::Display for VersionSet<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.range)
    }
}

impl<C: Candidate> PubGrubVersionSet for VersionSet<C> {
    type V = C;

    fn empty() -> VersionSet<C> {
        VersionSet {
            range: Range::none(),
            pre: Range::none(),
        }
    }

    fn singleton(c: C) -> VersionSet<C> {
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
        if !c.version().is_prerelease() {
            VersionSet {
                range: Range::exact(c.version().clone()),
                pre: Range::none(),
            }
        } else {
            VersionSet {
                range: Range::none(),
                pre: Range::exact(c.version().clone()),
            }
        }
    }

    fn complement(&self) -> VersionSet<C> {
        VersionSet {
            range: self.range.negate(),
            pre: self.pre.negate(),
        }
    }

    fn intersection(&self, other: &VersionSet<C>) -> VersionSet<C> {
        VersionSet {
            range: self.range.intersection(&other.range),
            pre: self.pre.intersection(&other.pre),
        }
    }

    fn contains(&self, c: &C) -> bool {
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
        if !c.version().is_prerelease() {
            self.range.contains(c.version())
        } else {
            self.pre.contains(c.version())
        }
    }
}

impl<C: Candidate> VersionSet<C> {
    pub(super) fn default() -> VersionSet<C> {
        VersionSet {
            range: Range::any(),
            pre: Range::none(),
        }
    }

    pub(super) fn exact(v: C::V) -> VersionSet<C> {
        VersionSet {
            range: Range::exact(v.clone()),
            pre: Range::exact(v),
        }
    }

    pub(super) fn between(left: C::V, right: C::V) -> VersionSet<C> {
        VersionSet {
            range: Range::between(left.clone(), right.clone()),
            pre: Range::between(left, right),
        }
    }

    pub(super) fn higher_than(v: C::V) -> VersionSet<C> {
        VersionSet {
            range: Range::higher_than(v.clone()),
            pre: Range::higher_than(v),
        }
    }

    pub(super) fn strictly_lower_than(v: C::V) -> VersionSet<C> {
        VersionSet {
            range: Range::strictly_lower_than(v.clone()),
            pre: Range::strictly_lower_than(v),
        }
    }

    pub(super) fn with_normal(&self, other: &VersionSet<C>) -> VersionSet<C> {
        VersionSet {
            range: self.range.intersection(&other.range),
            pre: self.pre.clone(),
        }
    }

    pub(super) fn with_pre(&self, other: &VersionSet<C>) -> VersionSet<C> {
        VersionSet {
            range: self.range.clone(),
            pre: self.pre.union(&other.pre),
        }
    }
}
