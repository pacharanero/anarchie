// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic publisher for policy-resolved CKM source packages.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{
    Inventory, KnowledgeManifest, PackageArchive, PackageError, Resolution, ResolutionError,
};

/// Evidence emitted beside every published International source package.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PublicationReport {
    pub package: PackageArchive,
    pub included_artefacts: usize,
    pub excluded_artefacts: usize,
    pub allowed_issues: usize,
}

#[derive(Debug, Error)]
pub enum PublicationError {
    #[error(transparent)]
    Inventory(#[from] super::InventoryError),
    #[error(transparent)]
    Resolution(#[from] ResolutionError),
    #[error(transparent)]
    Package(#[from] PackageError),
    #[error("knowledge resolution has {0} blocking issue(s)")]
    ResolutionBlocked(usize),
    #[error("source artefact changed after resolution: {path}")]
    SourceChanged { path: String },
    #[error("reading {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("serializing publication evidence: {0}")]
    Serialize(serde_json::Error),
}

/// Build a verified source package containing precisely one successful lock.
pub fn build_international_package(
    checkout: &Path,
    manifest: &KnowledgeManifest,
    version: &str,
    archive: &Path,
) -> Result<PublicationReport, PublicationError> {
    let inventory = Inventory::scan(checkout)?;
    let resolution = manifest.resolve(&inventory)?;
    if !resolution.is_success() || !resolution.issues.is_empty() {
        return Err(PublicationError::ResolutionBlocked(
            resolution.blockers().count().max(resolution.issues.len()),
        ));
    }
    let staging = archive.with_file_name(format!(
        ".ckm-international-{}.{}.tmp",
        std::process::id(),
        version
    ));
    fs::create_dir(&staging).map_err(|source| PublicationError::Io {
        path: staging.clone(),
        source,
    })?;
    let result = write_source_package(&staging, checkout, version, &inventory, &resolution)
        .and_then(|()| PackageArchive::build(&staging, archive).map_err(Into::into));
    let _ = fs::remove_dir_all(&staging);
    let package = result?;
    Ok(PublicationReport {
        package,
        included_artefacts: resolution.lock.artefacts.len(),
        excluded_artefacts: resolution
            .decisions
            .iter()
            .filter(|decision| !decision.included)
            .count(),
        allowed_issues: resolution
            .issues
            .iter()
            .filter(|issue| issue.allowed)
            .count(),
    })
}

fn write_source_package(
    root: &Path,
    checkout: &Path,
    version: &str,
    inventory: &Inventory,
    resolution: &Resolution,
) -> Result<(), PublicationError> {
    write(
        root.join("knowledge-package.toml"),
        format!("format_version = 1\nname = \"ckm-international\"\nversion = \"{version}\"\n")
            .as_bytes(),
    )?;
    for artefact in &resolution.lock.artefacts {
        let source = checkout.join(&artefact.path);
        let content = fs::read(&source).map_err(|error| PublicationError::Io {
            path: source,
            source: error,
        })?;
        if format!("sha256:{}", digest(&content)) != artefact.checksum {
            return Err(PublicationError::SourceChanged {
                path: artefact.path.clone(),
            });
        }
        let kind = match artefact.kind {
            super::ArtifactKind::Archetype => "archetypes",
            super::ArtifactKind::Template => "templates",
            super::ArtifactKind::Termset => "termsets",
        };
        write(
            root.join("artefacts").join(kind).join(&artefact.path),
            &content,
        )?;
    }
    write_json(root.join("provenance/source.json"), &inventory.source)?;
    write(
        root.join("provenance/knowledge.lock"),
        resolution.lock.to_toml()?.as_bytes(),
    )?;
    write_json(root.join("provenance/resolution.json"), resolution)?;
    write_json(
        root.join("provenance/inclusion-report.json"),
        &PublicationCounts::from(resolution),
    )
}

#[derive(Serialize)]
struct PublicationCounts {
    included_artefacts: usize,
    excluded_artefacts: usize,
    allowed_issues: usize,
}

impl From<&Resolution> for PublicationCounts {
    fn from(resolution: &Resolution) -> Self {
        Self {
            included_artefacts: resolution.lock.artefacts.len(),
            excluded_artefacts: resolution
                .decisions
                .iter()
                .filter(|decision| !decision.included)
                .count(),
            allowed_issues: resolution
                .issues
                .iter()
                .filter(|issue| issue.allowed)
                .count(),
        }
    }
}

fn write_json(path: PathBuf, value: &impl Serialize) -> Result<(), PublicationError> {
    let mut content = serde_json::to_vec_pretty(value).map_err(PublicationError::Serialize)?;
    content.push(b'\n');
    write(path, &content)
}

fn write(path: PathBuf, content: &[u8]) -> Result<(), PublicationError> {
    let parent = path.parent().expect("publication paths have parents");
    fs::create_dir_all(parent).map_err(|source| PublicationError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    fs::write(&path, content).map_err(|source| PublicationError::Io { path, source })
}

fn digest(content: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(content) {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}
