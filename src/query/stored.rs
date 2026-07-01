// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stored (named) queries: AQL registered once and run by name and version.
//!
//! Like templates, stored queries are **data, not code**: they live as plain
//! files under the deployment (`queries/<name>/<version>.aql`), git-friendly and
//! inspectable. openEHR distinguishes ad-hoc queries (text in the request) from
//! stored ones (registered, referenced by id); `anarchie` supports both. See
//! `specs/query-engine.md`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::query::aql::parse;
use crate::query::error::{QueryError, Result};

/// The default version assigned when a stored query is registered without one.
pub const DEFAULT_VERSION: &str = "1.0.0";

fn queries_dir(root: &Path) -> PathBuf {
    root.join("queries")
}

fn query_dir(root: &Path, name: &str) -> PathBuf {
    queries_dir(root).join(name)
}

fn query_path(root: &Path, name: &str, version: &str) -> PathBuf {
    query_dir(root, name).join(format!("{version}.aql"))
}

/// Register `aql` under `name` and `version`, validating it parses first.
/// Returns the stored `(name, version)`.
pub fn add(root: &Path, name: &str, version: Option<&str>, aql: &str) -> Result<(String, String)> {
    parse(aql).map_err(QueryError::Parse)?;
    let version = version.unwrap_or(DEFAULT_VERSION).to_string();
    let dir = query_dir(root, name);
    fs::create_dir_all(&dir)?;
    let body = if aql.ends_with('\n') {
        aql.to_string()
    } else {
        format!("{aql}\n")
    };
    fs::write(query_path(root, name, &version), body)?;
    Ok((name.to_string(), version))
}

/// One registered stored query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredQuery {
    pub name: String,
    pub version: String,
}

/// List all registered stored queries, sorted by name then version.
pub fn list(root: &Path) -> Result<Vec<StoredQuery>> {
    let dir = queries_dir(root);
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for name_entry in fs::read_dir(&dir)? {
        let name_entry = name_entry?;
        if !name_entry.path().is_dir() {
            continue;
        }
        let Some(name) = name_entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        for ver_entry in fs::read_dir(name_entry.path())? {
            let ver_entry = ver_entry?;
            if let Some(version) = ver_entry
                .file_name()
                .to_str()
                .and_then(|f| f.strip_suffix(".aql"))
            {
                out.push(StoredQuery {
                    name: name.clone(),
                    version: version.to_string(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
    Ok(out)
}

/// Fetch a stored query's AQL text by name, and version if given (otherwise the
/// lexically-highest version registered for that name).
pub fn get(root: &Path, name: &str, version: Option<&str>) -> Result<String> {
    let version = match version {
        Some(v) => v.to_string(),
        None => list(root)?
            .into_iter()
            .filter(|q| q.name == name)
            .map(|q| q.version)
            .next_back()
            .ok_or_else(|| QueryError::StoredQueryNotFound(name.to_string()))?,
    };
    let path = query_path(root, name, &version);
    if !path.exists() {
        return Err(QueryError::StoredQueryNotFound(format!("{name}/{version}")));
    }
    Ok(fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_list_get_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let aql = "SELECT c/name/value FROM COMPOSITION c";

        add(root, "all_names", None, aql).unwrap();
        add(root, "all_names", Some("2.0.0"), aql).unwrap();

        let listed = list(root).unwrap();
        assert_eq!(listed.len(), 2);

        // No version → highest (2.0.0).
        let latest = get(root, "all_names", None).unwrap();
        assert!(latest.starts_with("SELECT"));
        assert!(get(root, "all_names", Some("1.0.0")).is_ok());
        assert!(matches!(
            get(root, "missing", None),
            Err(QueryError::StoredQueryNotFound(_))
        ));
    }

    #[test]
    fn add_rejects_unparseable_aql() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            add(dir.path(), "bad", None, "NOT AQL AT ALL"),
            Err(QueryError::Parse(_))
        ));
    }
}
