// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use pubgrub::report::DerivationTree;

use crate::resolver::pubgrub::{Candidate, VersionSet};
use crate::resolver::types::Name;

pub type DerivedResult = DerivationTree<Name, VersionSet<Candidate>>;
