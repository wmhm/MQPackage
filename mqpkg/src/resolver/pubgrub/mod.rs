// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

pub(crate) use crate::resolver::pubgrub::candidate::Candidate;
pub(super) use crate::resolver::pubgrub::providers::RepositoryProvider;
pub(crate) use crate::resolver::pubgrub::types::DerivedResult;
pub(super) use crate::resolver::pubgrub::versionset::{
    Candidate as CandidateTrait, CandidateVersion, VersionSet,
};

mod candidate;
mod providers;
mod types;
mod versionset;
