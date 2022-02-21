// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use thiserror::Error;

use super::{Environment, EnvironmentError, PackageName};

#[derive(Error, Debug)]
pub enum InstallError {
    #[error("cannot add package")]
    CannotAddPackage { source: EnvironmentError },
}

pub fn install(env: &mut Environment, packages: &[PackageName]) -> Result<(), InstallError> {
    for package in packages {
        env.request(package)
            .map_err(|source| InstallError::CannotAddPackage { source })?;
    }

    Ok(())
}
