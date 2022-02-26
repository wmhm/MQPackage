// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::borrow::Borrow;
use std::fmt;

use log::{info, log_enabled, trace};
use pubgrub::error::PubGrubError;
use pubgrub::report::{DefaultStringReporter, DerivationTree, Reporter};
use pubgrub::solver::{
    choose_package_with_fewest_versions, resolve, Dependencies, DependencyProvider,
};
use pubgrub::type_aliases::{DependencyConstraints, SelectedDependencies};
use semver::Version;

use crate::errors::SolverError;
use crate::repository::Repository;
use crate::resolver::candidates::CandidateSet;
use crate::types::{PackageName, RequestedPackages};

pub(crate) use crate::resolver::candidates::Candidate;

mod candidates;

const LOGNAME: &str = "mqpkg::resolver";

// Note: The name used here **MUST** be an invalid name for packages to have,
//       if it's not, then our root package (which represents this stuff the
//       used has asked for) will collide with a real package.
const ROOT_NAME: &str = ":requested:";

// Note: The actual version doesn't matter here. This is just a marker so that
//       we can resolve the packages that the user has depended on.
const ROOT_VER: (u64, u64, u64) = (1, 0, 0);

pub type DerivedResult = DerivationTree<PackageName, CandidateSet>;

pub(crate) type SolverSolution = SelectedDependencies<PackageName, Candidate>;

impl SolverError {
    fn from_pubgrub(err: PubGrubError<PackageName, CandidateSet>) -> Self {
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
        let version = Candidate::new(Version::new(ROOT_VER.0, ROOT_VER.1, ROOT_VER.2));

        let resolver = InternalSolver {
            repository: &self.repository,
            root: package.clone(),
            requested: reqs,
            callback: Box::new(callback),
        };

        info!(target: LOGNAME, "resolving requested packages");

        let mut result =
            resolve(&resolver, package.clone(), version).map_err(SolverError::from_pubgrub)?;

        // Just remove our fake "root" package from our solution, since nothing but this
        // module should generally need to be aware it even exists.
        result.remove(&package);

        Ok(result)
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

impl<'r, 'c> DependencyProvider<PackageName, CandidateSet> for InternalSolver<'r, 'c> {
    fn should_cancel(&self) -> Result<(), Box<dyn std::error::Error>> {
        (self.callback)();
        Ok(())
    }

    fn choose_package_version<P: Borrow<PackageName>, U: Borrow<CandidateSet>>(
        &self,
        potential_packages: impl Iterator<Item = (P, U)>,
    ) -> Result<(P, Option<Candidate>), Box<dyn std::error::Error>> {
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

                versions.into_iter().map(Candidate::new)
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
        candidate: &Candidate,
    ) -> Result<Dependencies<PackageName, CandidateSet>, Box<dyn std::error::Error>> {
        let mut result = DependencyConstraints::<PackageName, CandidateSet>::default();

        let dependencies = if package == &self.root {
            self.requested.clone()
        } else {
            self.repository.dependencies(package, &candidate.version)
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
                candidate.version,
                req_str.join(", ")
            );
        }

        for (dep, req) in dependencies {
            result.insert(dep, CandidateSet::req(req));
        }

        Ok(Dependencies::Known(result))
    }
}
