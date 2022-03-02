// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;

use ::pubgrub::solver::{
    choose_package_with_fewest_versions, Dependencies as PDependencies, DependencyProvider,
};
use ::pubgrub::type_aliases::DependencyConstraints;
use log::{log_enabled, trace};

use crate::repository::Repository;
pub(crate) use crate::resolver::pubgrub::Candidate;
use crate::resolver::pubgrub::VersionSet;
use crate::resolver::types::WithDependencies;
pub(crate) use crate::resolver::types::{Name, Requirement};

const LOGNAME: &str = "mqpkg::resolver";

// Internal Solver keeps us from having to carefully maintain state, and let's us
// rely on the rust lifetime mechanic for that. We construct a new InternalSolver
// anytime that Solver::resolve is ran, which means that items that we don't want
// to persist between runs will only live on the InternalSolver. Anything we want
// to persist long term, lives on the Solver and gets passed into InternalSolver
// as a reference.
pub(in crate::resolver) struct RepositoryProvider<'r, 'c> {
    repository: &'r Repository,
    requested: HashMap<Name, Requirement>,
    callback: Box<dyn Fn() + 'c>,
}

impl<'r, 'c> RepositoryProvider<'r, 'c> {
    pub(in crate::resolver) fn new(
        repository: &'r Repository,
        requested: HashMap<Name, Requirement>,
        callback: Box<dyn Fn() + 'c>,
    ) -> RepositoryProvider<'r, 'c> {
        RepositoryProvider {
            repository,
            requested,
            callback,
        }
    }

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

impl<'r, 'c> DependencyProvider<Name, VersionSet<Candidate>> for RepositoryProvider<'r, 'c> {
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
