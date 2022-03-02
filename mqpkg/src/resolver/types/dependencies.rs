// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::collections::HashMap;
use std::fmt;

use dyn_clone::DynClone;

use crate::resolver::types::name::Name;
use crate::resolver::types::requirement::Requirement;

pub(crate) trait Dependencies: fmt::Debug + DynClone {
    fn get(&self) -> HashMap<Name, Requirement>;
}

dyn_clone::clone_trait_object!(Dependencies);

pub(in crate::resolver) trait WithDependencies {
    fn dependencies(&self) -> &dyn Dependencies;
}

#[derive(Debug, Clone)]
pub(crate) struct StaticDependencies {
    dependencies: HashMap<Name, Requirement>,
}

impl StaticDependencies {
    pub(crate) fn new<N: Into<Name>, R: Into<Requirement>>(
        dependencies: HashMap<N, R>,
    ) -> StaticDependencies {
        StaticDependencies {
            dependencies: dependencies
                .into_iter()
                .map(|(p, r)| (p.into(), r.into()))
                .collect(),
        }
    }
}

impl Dependencies for StaticDependencies {
    fn get(&self) -> HashMap<Name, Requirement> {
        self.dependencies.clone()
    }
}
