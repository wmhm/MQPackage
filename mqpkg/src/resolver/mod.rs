// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::borrow::Borrow;
use std::fmt;

use ::pubgrub::error::PubGrubError;
use ::pubgrub::report::{DefaultStringReporter, DerivationTree, Reporter};
use ::pubgrub::solver::{
    choose_package_with_fewest_versions, resolve, Dependencies as PDependencies, DependencyProvider,
};
use ::pubgrub::type_aliases::DependencyConstraints;
use log::{info, log_enabled, trace};

use crate::errors::SolverError;
use crate::repository::Repository;
use crate::resolver::pubgrub::Candidate as CandidateTrait;
use crate::resolver::semver::{VersionSet, WithDependencies};
use crate::types::{Package, PackageName, Packages, RequestedPackages, WithSource};

pub(crate) use crate::resolver::semver::{Candidate, Dependencies, Requirement};

mod pubgrub;
mod semver;

const LOGNAME: &str = "mqpkg::resolver";

// Note: The name used here **MUST** be an invalid name for packages to have,
//       if it's not, then our root package (which represents this stuff the
//       used has asked for) will collide with a real package.
const ROOT_NAME: &str = "requested packages";

pub type DerivedResult = DerivationTree<PackageName, VersionSet<Candidate>>;

impl SolverError {
    fn from_pubgrub(err: PubGrubError<PackageName, VersionSet<Candidate>>) -> Self {
        match err {
            PubGrubError::NoSolution(dt) => SolverError::NoSolution(Box::new(dt)),
            PubGrubError::DependencyOnTheEmptySet {
                package,
                version,
                dependent,
            } => SolverError::DependencyOnTheEmptySet {
                package,
                version: Box::new(version),
                dependent,
            },
            PubGrubError::SelfDependency { package, version } => SolverError::SelfDependency {
                package,
                version: Box::new(version),
            },
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
    ) -> Result<Packages, SolverError> {
        let package = PackageName::new(ROOT_NAME);
        let version = Candidate::root(reqs.clone());

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

        let packages: Packages = result
            .into_iter()
            .map(|(p, c)| (p.clone(), Package::new(p, c.version(), c.source().clone())))
            .collect();

        if log_enabled!(log::Level::Trace) {
            trace!(target: LOGNAME, "solution found");
            for pkg in packages.values() {
                trace!(target: LOGNAME, "solution package: {pkg}");
            }
        }

        Ok(packages)
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

impl<'r, 'c> DependencyProvider<PackageName, VersionSet<Candidate>> for InternalSolver<'r, 'c> {
    fn should_cancel(&self) -> Result<(), Box<dyn std::error::Error>> {
        (self.callback)();
        Ok(())
    }

    fn choose_package_version<P: Borrow<PackageName>, U: Borrow<VersionSet<Candidate>>>(
        &self,
        potential_packages: impl Iterator<Item = (P, U)>,
    ) -> Result<(P, Option<Candidate>), Box<dyn std::error::Error>> {
        let (package, version) = choose_package_with_fewest_versions(
            |package| {
                let candidates = if package == &self.root {
                    vec![Candidate::root(self.requested.clone())]
                } else {
                    self.repository.candidates(package)
                };

                if log_enabled!(log::Level::Trace) && package != &self.root {
                    let versions_str: Vec<String> =
                        candidates.iter().map(|v| v.to_string()).collect();
                    trace!(
                        target: LOGNAME,
                        "found versions for {}: [{}]",
                        package,
                        versions_str.join(", ")
                    );
                }

                candidates.into_iter()
            },
            potential_packages,
        );

        if log_enabled!(log::Level::Trace) {
            let version = version
                .clone()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "None".to_string());
            let version = if version.is_empty() {
                "".to_string()
            } else {
                format!(" ({})", version)
            };
            trace!(
                target: LOGNAME,
                "selected {}{} as next candidate",
                package.borrow(),
                version
            );
        }

        Ok((package, version))
    }

    fn get_dependencies(
        &self,
        package: &PackageName,
        candidate: &Candidate,
    ) -> Result<PDependencies<PackageName, VersionSet<Candidate>>, Box<dyn std::error::Error>> {
        if log_enabled!(log::Level::Trace) {
            let version = if package == &self.root {
                "".to_string()
            } else {
                format!(" ({})", candidate)
            };
            let req_str: Vec<String> = candidate
                .dependencies()
                .get()
                .iter()
                .map(|(k, v)| format!("{}({})", k, v))
                .collect();
            trace!(
                target: LOGNAME,
                "found dependencies for {}{}: [{}]",
                package,
                version,
                req_str.join(", ")
            );
        }

        let mut result = DependencyConstraints::<PackageName, VersionSet<Candidate>>::default();
        for (dep, req) in candidate.dependencies().get().iter() {
            result.insert(dep.clone(), req.into());
        }

        Ok(PDependencies::Known(result))
    }
}
