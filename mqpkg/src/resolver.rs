// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::borrow::Borrow;
use std::error::Error;

use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::solver::{
    choose_package_with_fewest_versions, resolve, Dependencies, DependencyConstraints,
    DependencyProvider, OfflineDependencyProvider,
};
use pubgrub::type_aliases::SelectedDependencies;
// use pubgrub::version::Version;

use crate::repository::Repository;
use crate::{PackageName, Version};

pub(crate) struct Solver {
    provider: OfflineDependencyProvider<PackageName, Version>,
    repository: Repository,
}

impl Solver {
    pub(crate) fn new(repository: Repository) -> Solver {
        let provider = OfflineDependencyProvider::new();
        Solver {
            provider,
            repository,
        }
    }

    pub(crate) fn resolve(
        &self,
        package: PackageName,
        version: Version,
    ) -> Result<SelectedDependencies<PackageName, Version>, PubGrubError<PackageName, Version>>
    {
        resolve(self, package, version)
    }
}

impl DependencyProvider<PackageName, Version> for Solver {
    fn choose_package_version<T: Borrow<PackageName>, U: Borrow<Range<Version>>>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> Result<(T, Option<Version>), Box<dyn Error>> {
        Ok(choose_package_with_fewest_versions(
            |p| self.repository.versions(p),
            potential_packages,
        ))
    }

    fn get_dependencies(
        &self,
        package: &PackageName,
        version: &Version,
    ) -> Result<Dependencies<PackageName, Version>, Box<dyn Error>> {
        let deps = DependencyConstraints::<PackageName, Version>::default();
        Ok(Dependencies::Known(deps))
    }
}
