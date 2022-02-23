// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::iter::Iterator;

use reqwest::blocking::Client as HTTPClient;
use semver::VersionReq;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

use crate::config;
use crate::{PackageName, Version};

#[derive(Deserialize, Debug)]
struct MetaData {
    name: String,
}

#[derive(Deserialize, Debug)]
struct PackageData {
    #[serde(default)]
    dependencies: HashMap<PackageName, VersionReq>,
    urls: Vec<Url>,
    digests: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct RepoData {
    meta: MetaData,
    packages: HashMap<PackageName, HashMap<Version, PackageData>>,
}

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error(transparent)]
    HTTPError(#[from] reqwest::Error),

    #[error("could not parse JSON data")]
    Deserialize(#[from] serde_json::Error),

    #[error("could not access local file")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug)]
pub(crate) struct Repository {
    client: HTTPClient,
    data: HashMap<String, RepoData>,
}

impl Repository {
    pub(crate) fn new() -> Result<Repository, RepositoryError> {
        let client = HTTPClient::builder().gzip(true).build()?;
        let data = HashMap::<String, RepoData>::new();

        Ok(Repository { client, data })
    }

    pub(crate) fn fetch(&mut self, repos: &[config::Repository]) -> Result<(), RepositoryError> {
        for repo in repos.iter() {
            let data: RepoData = match repo.url.scheme() {
                "file" => {
                    let file = File::open(repo.url.to_file_path().unwrap())?;
                    let reader = BufReader::new(file);

                    serde_json::from_reader(reader)?
                }
                _ => self
                    .client
                    .get(repo.url.clone())
                    .send()?
                    .error_for_status()?
                    .json()?,
            };
            self.data.insert(repo.name.clone(), data);
        }

        Ok(())
    }

    pub(crate) fn versions(&self, package: &PackageName) -> impl Iterator<Item = Version> {
        let mut versions = Vec::<Version>::new();

        versions.into_iter()
    }

    pub(crate) fn dependencies(
        &self,
        package: &PackageName,
        version: &Version,
    ) -> Result<HashMap<PackageName, VersionReq>, RepositoryError> {
        Ok(HashMap::new())
    }
}
