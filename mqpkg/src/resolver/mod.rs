// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::collections::HashMap;

use ::pubgrub::solver::resolve;
use log::{info, log_enabled, trace};

use crate::errors::SolverError;
use crate::repository::Repository;
pub(crate) use crate::resolver::pubgrub::{Candidate, DerivedResult};
use crate::resolver::pubgrub::{CandidateTrait, RepositoryProvider};
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

        let resolver = RepositoryProvider::new(
            &self.repository,
            reqs.into_iter()
                .map(|(p, r)| (p.into(), r.into()))
                .collect(),
            Box::new(callback),
        );

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
