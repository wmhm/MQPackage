// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::fmt;

use crate::types::PackageName;

// Note: The name used here **MUST** be an invalid name for packages to have,
//       if it's not, then our root package (which represents this stuff the
//       used has asked for) will collide with a real package.
const ROOT_NAME: &str = "requested packages";

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Name {
    root: bool,

    // For reasons I have yet to figure out, putting name not last breaks
    // resolving with pubgrub due to the derived hash implementation not
    // hashing it last.
    //
    // Why does pubgrub break in strange ways if this isn't hashed last?,
    // It is a mystery! Maybe some day I'll file a bug if I can ever get
    // a minimal reproducer and/or the versionset branch lands and I can
    // reproduce it with an actual release.
    name: PackageName,
}

impl Name {
    pub(in crate::resolver) fn new(name: PackageName) -> Name {
        assert!(name.to_string() != ROOT_NAME);
        Name { name, root: false }
    }

    pub(in crate::resolver) fn root() -> Name {
        Name {
            name: PackageName::new(ROOT_NAME),
            root: true,
        }
    }

    pub(in crate::resolver) fn is_root(&self) -> bool {
        self.root
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<PackageName> for Name {
    fn from(pn: PackageName) -> Name {
        Name::new(pn)
    }
}

impl From<Name> for PackageName {
    fn from(name: Name) -> PackageName {
        name.name
    }
}

impl AsRef<PackageName> for Name {
    fn as_ref(&self) -> &PackageName {
        &self.name
    }
}
