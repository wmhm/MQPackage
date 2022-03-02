// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;

use ::pubgrub::solver::{
    choose_package_with_fewest_versions, resolve, Dependencies as PDependencies, DependencyProvider,
};
use ::pubgrub::type_aliases::DependencyConstraints;
use log::{info, log_enabled, trace};

use crate::errors::SolverError;
use crate::repository::Repository;
pub(crate) use crate::resolver::pubgrub::{Candidate, DerivedResult};
use crate::resolver::pubgrub::{CandidateTrait, VersionSet};
use crate::resolver::types::WithDependencies;
pub(crate) use crate::resolver::types::{Name, Requirement, StaticDependencies};
use crate::types::{Package, Packages, WithSource};

mod errors;
mod pubgrub;
mod types;

const LOGNAME: &str = "mqpkg::resolver";

pub(crate) struct Solver {
    repository: Repository,
}

impl Solver {
    pub(crate) fn new(repository: Repository) -> Solver {
        Solver { repository }
    }

    pub(crate) fn resolve<N: Into<Name> + Clone, R: Into<Requirement> + Clone>(
        &self,
        reqs: HashMap<N, R>,
        callback: impl Fn(),
    ) -> Result<Packages, SolverError> {
        let package = Name::root();
        let version = Candidate::root(reqs.clone());

        let resolver = InternalSolver {
            repository: &self.repository,
            requested: reqs
                .into_iter()
                .map(|(p, r)| (p.into(), r.into()))
                .collect(),
            callback: Box::new(callback),
        };

        info!(target: LOGNAME, "resolving requested packages");

        let result = resolve(&resolver, package, version).map_err(SolverError::from_pubgrub)?;
        let packages: Packages = result
            .into_iter()
            // Filter out the root package from our results since nothing but this
            // module should even be aware it exists.
            .filter(|(p, _)| !p.is_root())
            // Turn our (Name, Candidate) into (PackageName, Package)
            .map(|(p, c)| {
                (
                    p.clone().into(),
                    Package::new(p, c.version(), c.source().clone()),
                )
            })
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
    requested: HashMap<Name, Requirement>,
    callback: Box<dyn Fn() + 'c>,
}

impl<'r, 'c> InternalSolver<'r, 'c> {
    fn list_versions(&self, package: &Name) -> std::vec::IntoIter<Candidate> {
        let candidates = if package.is_root() {
            vec![Candidate::root(self.requested.clone())]
        } else {
            self.repository.candidates(package)
        };

        if log_enabled!(log::Level::Trace) && !package.is_root() {
            let versions_str: Vec<String> = candidates.iter().map(|v| v.to_string()).collect();
            trace!(
                target: LOGNAME,
                "found versions for {}: [{}]",
                package,
                versions_str.join(", ")
            );
        }

        candidates.into_iter()
    }
}

impl<'r, 'c> DependencyProvider<Name, VersionSet<Candidate>> for InternalSolver<'r, 'c> {
    fn should_cancel(&self) -> Result<(), Box<dyn std::error::Error>> {
        (self.callback)();
        Ok(())
    }

    fn choose_package_version<P: Borrow<Name>, U: Borrow<VersionSet<Candidate>>>(
        &self,
        potential_packages: impl Iterator<Item = (P, U)>,
    ) -> Result<(P, Option<Candidate>), Box<dyn std::error::Error>> {
        let (package, version) =
            choose_package_with_fewest_versions(|p| self.list_versions(p), potential_packages);

        if log_enabled!(log::Level::Trace) {
            let version = version
                .clone()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "None".to_string());
            trace!(
                target: LOGNAME,
                "selected {}{} as next candidate",
                package.borrow(),
                version_str(&version, version.is_empty())
            );
        }

        Ok((package, version))
    }

    fn get_dependencies(
        &self,
        package: &Name,
        candidate: &Candidate,
    ) -> Result<PDependencies<Name, VersionSet<Candidate>>, Box<dyn std::error::Error>> {
        match candidate.dependencies().get() {
            None => {
                trace!(
                    target: LOGNAME,
                    "could not determine dependencies for {package}"
                );

                Ok(PDependencies::Unknown)
            }
            Some(deps) => {
                if log_enabled!(log::Level::Trace) {
                    let req_str: Vec<String> =
                        deps.iter().map(|(k, v)| format!("{}({})", k, v)).collect();
                    trace!(
                        target: LOGNAME,
                        "found dependencies for {}{}: [{}]",
                        package,
                        version_str(candidate, package.is_root()),
                        req_str.join(", ")
                    );
                }

                let mut result = DependencyConstraints::<Name, VersionSet<Candidate>>::default();
                for (dep, req) in deps.iter() {
                    result.insert(dep.clone(), req.into());
                }
                Ok(PDependencies::Known(result))
            }
        }
    }
}

fn version_str<V: fmt::Display>(version: &V, should_display: bool) -> String {
    if should_display {
        format!(" ({})", version)
    } else {
        "".to_string()
    }
}
