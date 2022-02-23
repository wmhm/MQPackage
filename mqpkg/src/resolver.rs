// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::borrow::Borrow;
use std::collections::HashMap;

use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::DerivationTree;
use pubgrub::solver::{
    choose_package_with_fewest_versions, resolve, Dependencies, DependencyConstraints,
    DependencyProvider,
};
use pubgrub::type_aliases::SelectedDependencies;
use semver::VersionReq;
use thiserror::Error;

use crate::repository::Repository;
use crate::{PackageName, Version};

// Note: The name used here **MUST** be an invalid name for packages to have,
//       if it's not, then our root package (which represents this stuff the
//       used has asked for) will collide with a real package.
const ROOT_NAME: &str = ":root:";
// Note: The actual version doesn't matter here. This is just a marker so that
//       we can resolve the packages that the user has depended on.
const ROOT_VER: (u64, u64, u64) = (0, 0, 0);

#[derive(Error, Debug)]
pub enum SolverError {
    #[error("No solution")]
    NoSolution(Box<DerivationTree<PackageName, Version>>),
}

impl SolverError {
    fn from_pubgrub(err: PubGrubError<PackageName, Version>) -> Self {
        match err {
            PubGrubError::NoSolution(dt) => SolverError::NoSolution(Box::new(dt)),
            _ => panic!("unhandled error"),
        }
    }
}

pub(crate) type Solution = SelectedDependencies<PackageName, Version>;

pub(crate) type Requested = HashMap<PackageName, VersionReq>;

pub(crate) struct Solver {
    repository: Repository,
}

impl Solver {
    pub(crate) fn new(repository: Repository) -> Solver {
        Solver { repository }
    }

    pub(crate) fn resolve(&self, reqs: Requested) -> Result<Solution, SolverError> {
        let package = PackageName(ROOT_NAME.to_string());
        let version = Version::new(ROOT_VER.0, ROOT_VER.1, ROOT_VER.2);

        let resolver = InternalSolver {
            repository: &self.repository,
            root: package.clone(),
            requested: reqs,
        };

        resolve(&resolver, package, version).map_err(SolverError::from_pubgrub)
    }
}

// Internal Solver keeps us from having to carefully maintain state, and let's us
// rely on the rust lifetime mechanic for that. We construct a new InternalSolver
// anytime that Solver::resolve is ran, which means that items that we don't want
// to persist between runs will only live on the InternalSolver. Anything we want
// to persist long term, lives on the Solver and gets passed into InternalSolver
// as a reference.
struct InternalSolver<'r> {
    repository: &'r Repository,
    root: PackageName,
    requested: Requested,
}

impl<'r> DependencyProvider<PackageName, Version> for InternalSolver<'r> {
    fn choose_package_version<T: Borrow<PackageName>, U: Borrow<Range<Version>>>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> Result<(T, Option<Version>), Box<dyn std::error::Error>> {
        Ok(choose_package_with_fewest_versions(
            |p| self.repository.versions(p),
            potential_packages,
        ))
    }

    fn get_dependencies(
        &self,
        package: &PackageName,
        version: &Version,
    ) -> Result<Dependencies<PackageName, Version>, Box<dyn std::error::Error>> {
        let mut result = DependencyConstraints::<PackageName, Version>::default();

        let dependencies = if package == &self.root {
            self.requested.clone()
        } else {
            self.repository.dependencies(package, version)
        };

        for (dep, req) in dependencies {
            for comp in req.comparators.iter() {
                match convert(comp) {
                    Ok(new) => merge(&mut result, &dep, new),
                    Err(e) => match e {
                        ComparatorError::InvalidVersion => {
                            panic!("version with no minor but a patch: {:?}", req)
                        }
                        ComparatorError::UnknownOperator => {
                            panic!("unknown semver operator: {:?}", req)
                        }
                        ComparatorError::InvalidWildcard => {
                            panic!("invalid wildcard: {:?}", req)
                        }
                    },
                }
            }
        }

        Ok(Dependencies::Known(result))
    }
}

fn merge(
    deps: &mut DependencyConstraints<PackageName, Version>,
    package: &PackageName,
    new: Range<Version>,
) {
    let existing = deps.get(package);
    let merged = match existing {
        Some(other) => new.intersection(other),
        None => new,
    };
    deps.insert(package.clone(), merged);
}

#[derive(Error, Debug)]
pub enum ComparatorError {
    #[error("version with no minor but a patch")]
    InvalidVersion,

    #[error("unknown operator")]
    UnknownOperator,

    #[error("wildcard with patch version")]
    InvalidWildcard,
}

fn convert(comp: &semver::Comparator) -> Result<Range<Version>, ComparatorError> {
    let major = comp.major;
    match comp.op {
        semver::Op::Exact => match (comp.minor, comp.patch) {
            //  =I.J.K — exactly the version I.J.K
            (Some(minor), Some(patch)) => Ok(Range::exact(Version::new(major, minor, patch))),
            // =I.J — equivalent to >=I.J.0, <I.(J+1).0
            (Some(minor), None) => Ok(Range::between(
                Version::new(major, minor, 0),
                Version::new(major, minor + 1, 0),
            )),
            // =I — equivalent to >=I.0.0, <(I+1).0.0
            (None, None) => Ok(Range::between(
                Version::new(major, 0, 0),
                Version::new(major + 1, 0, 0),
            )),
            _ => Err(ComparatorError::InvalidVersion),
        },
        semver::Op::Greater => match (comp.minor, comp.patch) {
            // >I.J.K
            (Some(minor), Some(patch)) => {
                Ok(Range::higher_than(Version::new(major, minor, patch + 1)))
            }
            // >I.J — equivalent to >=I.(J+1).0
            (Some(minor), None) => Ok(Range::higher_than(Version::new(major, minor + 1, 0))),
            // >I — equivalent to >=(I+1).0.0
            (None, None) => Ok(Range::higher_than(Version::new(major + 1, 0, 0))),
            _ => Err(ComparatorError::InvalidVersion),
        },
        semver::Op::GreaterEq => match (comp.minor, comp.patch) {
            //  >=I.J.K
            (Some(minor), Some(patch)) => Ok(Range::higher_than(Version::new(major, minor, patch))),
            // >=I.J — equivalent to >=I.J.0
            (Some(minor), None) => Ok(Range::higher_than(Version::new(major, minor, 0))),
            // >=I — equivalent to >=I.0.0
            (None, None) => Ok(Range::higher_than(Version::new(major, 0, 0))),
            _ => Err(ComparatorError::InvalidVersion),
        },
        semver::Op::Less => match (comp.minor, comp.patch) {
            // <I.J.K
            (Some(minor), Some(patch)) => Ok(Range::strictly_lower_than(Version::new(
                major, minor, patch,
            ))),
            // <I.J — equivalent to <I.J.0
            (Some(minor), None) => Ok(Range::strictly_lower_than(Version::new(major, minor, 0))),
            // <I — equivalent to <I.0.0
            (None, None) => Ok(Range::strictly_lower_than(Version::new(major, 0, 0))),
            _ => Err(ComparatorError::InvalidVersion),
        },
        semver::Op::LessEq => match (comp.minor, comp.patch) {
            // <=I.J.K — equivalent to <I.J.(K+1)
            (Some(minor), Some(patch)) => Ok(Range::strictly_lower_than(Version::new(
                major,
                minor,
                patch + 1,
            ))),
            // <=I.J — equivalent to <I.(J+1).0
            (Some(minor), None) => Ok(Range::strictly_lower_than(Version::new(
                major,
                minor + 1,
                0,
            ))),
            // <=I — equivalent to <(I+1).0.0
            (None, None) => Ok(Range::strictly_lower_than(Version::new(major + 1, 0, 0))),
            _ => Err(ComparatorError::InvalidVersion),
        },
        semver::Op::Tilde => match (comp.minor, comp.patch) {
            // ~I.J.K — equivalent to >=I.J.K, <I.(J+1).0
            (Some(minor), Some(patch)) => Ok(Range::between(
                Version::new(major, minor, patch),
                Version::new(major, minor + 1, 0),
            )),
            // ~I.J — equivalent to =I.J
            (Some(minor), None) => Ok(Range::between(
                Version::new(major, minor, 0),
                Version::new(major, minor + 1, 0),
            )),
            // ~I — equivalent to =I
            (None, None) => Ok(Range::between(
                Version::new(major, 0, 0),
                Version::new(major + 1, 0, 0),
            )),
            _ => Err(ComparatorError::InvalidVersion),
        },
        semver::Op::Caret => match (comp.minor, comp.patch) {
            (Some(minor), Some(patch)) => {
                if major > 0 {
                    // ^I.J.K (for I>0) — equivalent to >=I.J.K, <(I+1).0.0
                    Ok(Range::between(
                        Version::new(major, minor, patch),
                        Version::new(major + 1, 0, 0),
                    ))
                } else if minor > 0 {
                    // ^0.J.K (for J>0) — equivalent to >=0.J.K, <0.(J+1).0
                    assert!(major == 0);
                    Ok(Range::between(
                        Version::new(0, minor, patch),
                        Version::new(0, minor + 1, 0),
                    ))
                } else {
                    // ^0.0.K — equivalent to =0.0.K
                    assert!(major == 0 && minor == 0);
                    Ok(Range::exact(Version::new(0, 0, patch)))
                }
            }
            (Some(minor), None) => {
                if major > 0 || minor > 0 {
                    // ^I.J (for I>0 or J>0) — equivalent to ^I.J.0
                    Ok(Range::between(
                        Version::new(major, minor, 0),
                        Version::new(major + 1, 0, 0),
                    ))
                } else {
                    // ^0.0 — equivalent to =0.0
                    assert!(major == 0 && minor == 0);
                    Ok(Range::between(
                        Version::new(major, minor, 0),
                        Version::new(major, minor + 1, 0),
                    ))
                }
            }
            // ^I — equivalent to =I
            (None, None) => Ok(Range::between(
                Version::new(major, 0, 0),
                Version::new(major + 1, 0, 0),
            )),
            _ => Err(ComparatorError::InvalidVersion),
        },
        semver::Op::Wildcard => match (comp.minor, comp.patch) {
            (Some(_), Some(_)) => Err(ComparatorError::InvalidWildcard),
            (Some(minor), None) => Ok(Range::between(
                Version::new(major, minor, 0),
                Version::new(major, minor + 1, 0),
            )),
            (None, None) => Ok(Range::between(
                Version::new(major, 0, 0),
                Version::new(major + 1, 0, 0),
            )),
            _ => Err(ComparatorError::InvalidVersion),
        },
        _ => Err(ComparatorError::UnknownOperator),
    }
}
