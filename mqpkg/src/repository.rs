// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;

use indexmap::IndexMap;
use log::info;
use reqwest::blocking::Client as HTTPClient;
use semver::VersionReq;
use serde::Deserialize;
use url::Url;

use crate::config;
use crate::errors::RepositoryError;
use crate::progress::Progress;
use crate::types::{PackageName, Version};

const LOGNAME: &str = "mqpkg::repository";

type Result<T, E = RepositoryError> = core::result::Result<T, E>;

#[derive(Deserialize, Debug)]
struct MetaData {
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize, Debug)]
struct Release {
    #[serde(default)]
    dependencies: HashMap<PackageName, VersionReq>,
    #[serde(rename = "urls")]
    _urls: Vec<Url>,
    #[serde(rename = "digests")]
    _digests: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct RepoData {
    #[serde(rename = "meta")]
    _meta: MetaData,
    packages: HashMap<PackageName, HashMap<Version, Release>>,
}

#[derive(Debug)]
pub(crate) struct Repository<'p, T> {
    progress: Progress<'p, T>,
    client: HTTPClient,
    data: IndexMap<String, RepoData>,
}

impl<'p, T> Repository<'p, T> {
    pub(crate) fn new(progress: Progress<'p, T>) -> Result<Repository<'p, T>> {
        let client = HTTPClient::builder().gzip(true).build()?;
        let data = IndexMap::<String, RepoData>::new();

        Ok(Repository {
            client,
            data,
            progress,
        })
    }

    pub(crate) fn fetch(mut self, repos: &[config::Repository]) -> Result<Repository<'p, T>> {
        info!(target: LOGNAME, "fetching package metadata");
        let bar = self.progress.bar(repos.len().try_into().unwrap());
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
            bar.update(1);
        }
        bar.finish();

        Ok(self)
    }

    pub(crate) fn versions(&self, package: &PackageName) -> Vec<Version> {
        let mut versions = Vec::<Version>::new();
        let mut seen = HashSet::<Version>::new();

        for data in self.data.values() {
            if let Some(packages) = data.packages.get(package) {
                for version in packages.keys() {
                    if seen.get(version).is_none() {
                        seen.insert(version.clone());
                        versions.push(version.clone());
                    }
                }
            }
        }

        // We want to put the newest version first, this will make sure that our resolver
        // will do intelligent things, like trying the newest version.
        versions.sort_unstable_by(|l, r| l.cmp(r).reverse());
        versions
    }

    pub(crate) fn dependencies(
        &self,
        package: &PackageName,
        version: &Version,
    ) -> HashMap<PackageName, VersionReq> {
        let mut deps = HashMap::new();

        for data in self.data.values() {
            if let Some(packages) = data.packages.get(package) {
                if let Some(release) = packages.get(version) {
                    for (key, value) in release.dependencies.iter() {
                        deps.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        deps
    }
}
