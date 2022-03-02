// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::fmt;

use pubgrub::error::PubGrubError;
use pubgrub::report::{DefaultStringReporter, Reporter};

use crate::errors::SolverError;
use crate::resolver::pubgrub::{Candidate, DerivedResult, VersionSet};
use crate::resolver::types::Name;

impl SolverError {
    pub(super) fn from_pubgrub(err: PubGrubError<Name, VersionSet<Candidate>>) -> Self {
        match err {
            PubGrubError::NoSolution(dt) => SolverError::NoSolution(Box::new(dt)),
            PubGrubError::DependencyOnTheEmptySet {
                package,
                version,
                dependent,
            } => SolverError::DependencyOnTheEmptySet {
                package: package.into(),
                version: Box::new(version),
                dependent: dependent.into(),
            },
            PubGrubError::SelfDependency { package, version } => SolverError::SelfDependency {
                package: package.into(),
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
