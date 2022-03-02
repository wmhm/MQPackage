// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

pub(crate) use crate::resolver::types::dependencies::Dependencies;
pub(crate) use crate::resolver::types::name::Name;
pub(crate) use crate::resolver::types::requirement::Requirement;

pub(super) use crate::resolver::types::dependencies::{StaticDependencies, WithDependencies};
pub(super) use crate::resolver::types::version::Version;

mod dependencies;
mod name;
mod requirement;
mod version;
