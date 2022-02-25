// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::borrow::Borrow;
use std::fmt;

use log::{info, log_enabled, trace};
use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::solver::{
    choose_package_with_fewest_versions, resolve, Dependencies, DependencyConstraints,
    DependencyProvider,
};
use pubgrub::version::Version as RVersion;
use thiserror::Error;

use crate::errors::SolverError;
use crate::repository::Repository;
use crate::types::{DerivedResult, PackageName, RequestedPackages, SolverSolution, Version};

const LOGNAME: &str = "mqpkg::resolver";

// Note: The name used here **MUST** be an invalid name for packages to have,
//       if it's not, then our root package (which represents this stuff the
//       used has asked for) will collide with a real package.
const ROOT_NAME: &str = ":requested:";

// Note: The actual version doesn't matter here. This is just a marker so that
//       we can resolve the packages that the user has depended on.
const ROOT_VER: (u64, u64, u64) = (1, 0, 0);

impl SolverError {
    fn from_pubgrub(err: PubGrubError<PackageName, Version>) -> Self {
        match err {
            PubGrubError::NoSolution(dt) => SolverError::NoSolution(Box::new(dt)),
            PubGrubError::DependencyOnTheEmptySet {
                package,
                version,
                dependent,
            } => SolverError::DependencyOnTheEmptySet {
                package,
                version,
                dependent,
            },
            PubGrubError::SelfDependency { package, version } => {
                SolverError::SelfDependency { package, version }
            }
            PubGrubError::Failure(s) => SolverError::Failure(s),
            PubGrubError::ErrorRetrievingDependencies { .. } => SolverError::Impossible,
            PubGrubError::ErrorChoosingPackageVersion(_) => SolverError::Impossible,
            PubGrubError::ErrorInShouldCancel(_) => SolverError::Impossible,
        }
    }

    pub fn humanized<S: Into<String>>(msg: S, dt: DerivedResult) -> HumanizedNoSolutionError {
        HumanizedNoSolutionError {
            msg: msg.into(),
            dt,
        }
    }
}

impl RVersion for Version {
    fn lowest() -> Version {
        Version::new(0, 0, 0)
    }

    fn bump(&self) -> Version {
        Version::new(self.major(), self.minor(), self.patch() + 1)
    }
}

#[derive(Debug)]
pub struct HumanizedNoSolutionError {
    msg: String,
    dt: DerivedResult,
}

impl fmt::Display for HumanizedNoSolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n\n", self.msg.as_str())?;
        writeln!(f, "{}", DefaultStringReporter::report(&self.dt))?;

        Ok(())
    }
}

impl std::error::Error for HumanizedNoSolutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

pub(crate) struct Solver {
    repository: Repository,
}

impl Solver {
    pub(crate) fn new(repository: Repository) -> Solver {
        Solver { repository }
    }

    pub(crate) fn resolve(
        &self,
        reqs: RequestedPackages,
        callback: impl Fn(),
    ) -> Result<SolverSolution, SolverError> {
        let package = PackageName::new(ROOT_NAME);
        let version = Version::new(ROOT_VER.0, ROOT_VER.1, ROOT_VER.2);

        let resolver = InternalSolver {
            repository: &self.repository,
            root: package.clone(),
            requested: reqs,
            callback: Box::new(callback),
        };

        info!(target: LOGNAME, "resolving requested packages");

        resolve(&resolver, package, version).map_err(SolverError::from_pubgrub)
    }
}

// Internal Solver keeps us from having to carefully maintain state, and let's us
// rely on the rust lifetime mechanic for that. We construct a new InternalSolver
// anytime that Solver::resolve is ran, which means that items that we don't want
// to persist between runs will only live on the InternalSolver. Anything we want
// to persist long term, lives on the Solver and gets passed into InternalSolver
// as a reference.
struct InternalSolver<'r, 'c> {
    repository: &'r Repository,
    root: PackageName,
    requested: RequestedPackages,
    callback: Box<dyn Fn() + 'c>,
}

impl<'r, 'c> DependencyProvider<PackageName, Version> for InternalSolver<'r, 'c> {
    fn should_cancel(&self) -> Result<(), Box<dyn std::error::Error>> {
        (self.callback)();
        Ok(())
    }

    fn choose_package_version<P: Borrow<PackageName>, U: Borrow<Range<Version>>>(
        &self,
        potential_packages: impl Iterator<Item = (P, U)>,
    ) -> Result<(P, Option<Version>), Box<dyn std::error::Error>> {
        let (package, version) = choose_package_with_fewest_versions(
            |package| {
                let versions = if package == &self.root {
                    vec![Version::new(ROOT_VER.0, ROOT_VER.1, ROOT_VER.2)]
                } else {
                    self.repository.versions(package)
                };

                if log_enabled!(log::Level::Trace) {
                    let versions_str: Vec<String> =
                        versions.iter().map(|v| v.to_string()).collect();
                    trace!(
                        target: LOGNAME,
                        "found versions for {}: [{}]",
                        package,
                        versions_str.join(", ")
                    );
                }

                versions.into_iter()
            },
            potential_packages,
        );

        if log_enabled!(log::Level::Trace) {
            trace!(
                target: LOGNAME,
                "selected {} ({}) as next candidate",
                package.borrow(),
                version
                    .clone()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "None".to_string())
            );
        }

        Ok((package, version))
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

        if log_enabled!(log::Level::Trace) {
            let req_str: Vec<String> = dependencies
                .iter()
                .map(|(k, v)| format!("{}({})", k, v))
                .collect();
            trace!(
                target: LOGNAME,
                "found dependencies for {} ({}): [{}]",
                package,
                version,
                req_str.join(", ")
            );
        }

        // Convert all of our semver::VersionReq into pubgrub::Range
        for (dep, req) in dependencies {
            if req.comparators.is_empty() {
                merge(&mut result, &dep, Range::any())
            } else {
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
enum ComparatorError {
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
