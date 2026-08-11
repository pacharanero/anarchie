// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic inventory of an openEHR CKM mirror checkout.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const FORMAT_VERSION: u32 = 1;

/// A complete point-in-time inventory of one CKM mirror checkout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    pub format_version: u32,
    pub source: SourceEvidence,
    pub summary: InventorySummary,
    pub artifacts: Vec<Artifact>,
    pub dependency_issues: Vec<DependencyIssue>,
}

/// Source-level evidence shared by every inventoried artefact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEvidence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_dirty: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_license_sha256: Option<String>,
}

/// Deterministic aggregate counts over the inventory.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySummary {
    pub artifacts: usize,
    pub archetypes: usize,
    pub published_archetypes: usize,
    pub templates: usize,
    pub termsets: usize,
    pub lifecycle_states: BTreeMap<String, usize>,
    pub licenses: BTreeMap<String, usize>,
    pub parse_issues: BTreeMap<String, usize>,
    pub dependency_issues: BTreeMap<String, usize>,
}

/// One source knowledge artefact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub kind: ArtifactKind,
    pub path: String,
    pub origin: String,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rm_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adl_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_spdx: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hard_dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slot_constraints: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parse_issues: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Archetype,
    Template,
    Termset,
}

/// One unresolved or ambiguous hard-dependency edge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyIssue {
    pub kind: DependencyIssueKind,
    pub artifact_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    pub dependency: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyIssueKind {
    MissingHardDependency,
    MajorVersionMismatch,
    AmbiguousHardDependency,
    DuplicateArtifactId,
}

impl DependencyIssueKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::MissingHardDependency => "missing_hard_dependency",
            Self::MajorVersionMismatch => "major_version_mismatch",
            Self::AmbiguousHardDependency => "ambiguous_hard_dependency",
            Self::DuplicateArtifactId => "duplicate_artifact_id",
        }
    }
}

#[derive(Debug, Error)]
pub enum InventoryError {
    #[error("CKM checkout `{0}` has neither a local/ nor remote/ directory")]
    NotCkmCheckout(PathBuf),
    #[error("reading {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("path `{0}` is not valid UTF-8")]
    NonUtf8Path(PathBuf),
}

impl Inventory {
    /// Inventory a local CKM mirror checkout without modifying it.
    pub fn scan(root: &Path) -> Result<Self, InventoryError> {
        let has_local = root.join("local").is_dir();
        let has_remote = root.join("remote").is_dir();
        if !has_local && !has_remote {
            return Err(InventoryError::NotCkmCheckout(root.to_path_buf()));
        }

        let mut paths = Vec::new();
        if has_local {
            collect_source_files(root, &root.join("local"), &mut paths)?;
        }
        if has_remote {
            collect_source_files(root, &root.join("remote"), &mut paths)?;
        }
        paths.sort();

        let mut artifacts = Vec::with_capacity(paths.len());
        for path in paths {
            artifacts.push(parse_artifact(root, &path)?);
        }
        artifacts.sort_by(|a, b| a.path.cmp(&b.path));

        let dependency_issues = resolve_dependencies(&artifacts);
        let summary = summarize(&artifacts, &dependency_issues);
        let source = source_evidence(root)?;

        Ok(Self {
            format_version: FORMAT_VERSION,
            source,
            summary,
            artifacts,
            dependency_issues,
        })
    }
}

fn collect_source_files(
    root: &Path,
    directory: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), InventoryError> {
    let entries = fs::read_dir(directory).map_err(|source| InventoryError::Io {
        path: directory.to_path_buf(),
        source,
    })?;
    let mut entries: Vec<_> =
        entries
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| InventoryError::Io {
                path: directory.to_path_buf(),
                source,
            })?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| InventoryError::Io {
            path: path.clone(),
            source,
        })?;
        if file_type.is_dir() {
            collect_source_files(root, &path, output)?;
        } else if file_type.is_file() && is_source_file(&path) {
            let _ = relative_path(root, &path)?;
            output.push(path);
        }
    }
    Ok(())
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("adl" | "oet" | "xml")
    )
}

fn parse_artifact(root: &Path, path: &Path) -> Result<Artifact, InventoryError> {
    let bytes = fs::read(path).map_err(|source| InventoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let relative = relative_path(root, path)?;
    let origin = artifact_origin(&relative);
    let sha256 = digest(&bytes);

    match path.extension().and_then(|extension| extension.to_str()) {
        Some("adl") => Ok(parse_archetype(&relative, origin, sha256, &text)),
        Some("oet") => Ok(parse_template(&relative, origin, sha256, &text)),
        _ => Ok(parse_termset(&relative, origin, sha256, &text)),
    }
}

fn parse_archetype(path: &str, origin: String, sha256: String, text: &str) -> Artifact {
    let id = text
        .lines()
        .take(12)
        .map(str::trim)
        .find(|line| line.starts_with("openEHR-"))
        .map(str::to_string);
    let adl_version = extract_between(
        text.lines().next().unwrap_or_default(),
        "adl_version=",
        &[';', ',', ')'],
    );
    let lifecycle_state = extract_odin_scalar(text, "lifecycle_state");
    let revision = extract_odin_map_value(text, "revision");
    let license = extract_odin_map_value(text, "licence");
    let license_spdx = license.as_deref().and_then(normalize_license);
    let languages = extract_languages(text);
    let hard_dependencies = extract_adl_dependencies(text);
    let slot_constraints = extract_slot_constraints(text);
    let mut parse_issues = Vec::new();
    if id.is_none() {
        parse_issues.push("missing archetype identifier".to_string());
    }
    if lifecycle_state.is_none() {
        parse_issues.push("missing lifecycle_state".to_string());
    }
    if revision.is_none() {
        parse_issues.push("missing revision metadata".to_string());
    }
    if license.is_none() {
        parse_issues.push("missing licence metadata".to_string());
    }

    Artifact {
        kind: ArtifactKind::Archetype,
        path: path.to_string(),
        origin,
        sha256,
        rm_type: id.as_deref().and_then(rm_type),
        id,
        name: None,
        adl_version,
        revision,
        lifecycle_state,
        license,
        license_spdx,
        languages,
        hard_dependencies,
        slot_constraints,
        parse_issues,
    }
}

fn parse_template(path: &str, origin: String, sha256: String, text: &str) -> Artifact {
    let id = extract_xml_element(text, "id");
    let name = extract_xml_element(text, "name");
    let lifecycle_state = extract_xml_element(text, "lifecycle_state");
    let license = extract_xml_map_value(text, "licence").filter(|value| !value.trim().is_empty());
    let license_spdx = license.as_deref().and_then(normalize_license);
    let hard_dependencies = extract_xml_attributes(text, "archetype_id");
    let mut parse_issues = Vec::new();
    if id.is_none() {
        parse_issues.push("missing template identifier".to_string());
    }
    if name.is_none() {
        parse_issues.push("missing template name".to_string());
    }
    if lifecycle_state.is_none() {
        parse_issues.push("missing lifecycle_state".to_string());
    }
    if license.is_none() {
        parse_issues.push("missing licence metadata".to_string());
    }

    Artifact {
        kind: ArtifactKind::Template,
        path: path.to_string(),
        origin,
        sha256,
        id,
        name,
        rm_type: None,
        adl_version: None,
        revision: None,
        lifecycle_state,
        license,
        license_spdx,
        languages: Vec::new(),
        hard_dependencies,
        slot_constraints: Vec::new(),
        parse_issues,
    }
}

fn parse_termset(path: &str, origin: String, sha256: String, text: &str) -> Artifact {
    let name = extract_xml_element(text, "name").or_else(|| {
        Path::new(path)
            .file_stem()
            .and_then(|value| value.to_str())
            .map(str::to_string)
    });
    Artifact {
        kind: ArtifactKind::Termset,
        path: path.to_string(),
        origin,
        sha256,
        id: name.clone(),
        name,
        rm_type: None,
        adl_version: None,
        revision: None,
        lifecycle_state: None,
        license: None,
        license_spdx: None,
        languages: Vec::new(),
        hard_dependencies: Vec::new(),
        slot_constraints: Vec::new(),
        parse_issues: vec!["termset licence inherited from repository evidence".to_string()],
    }
}

fn resolve_dependencies(artifacts: &[Artifact]) -> Vec<DependencyIssue> {
    let mut archetypes: BTreeMap<&str, Vec<&Artifact>> = BTreeMap::new();
    for artifact in artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::Archetype)
    {
        if let Some(id) = artifact.id.as_deref() {
            archetypes.entry(id).or_default().push(artifact);
        }
    }

    let mut issues = Vec::new();
    for (id, matches) in &archetypes {
        if matches.len() > 1 {
            for artifact in matches {
                issues.push(DependencyIssue {
                    kind: DependencyIssueKind::DuplicateArtifactId,
                    artifact_path: artifact.path.clone(),
                    artifact_id: artifact.id.clone(),
                    dependency: (*id).to_string(),
                    candidates: matches.iter().map(|item| item.path.clone()).collect(),
                });
            }
        }
    }

    for artifact in artifacts {
        for dependency in &artifact.hard_dependencies {
            match archetypes.get(dependency.as_str()) {
                Some(matches) if matches.len() == 1 => {}
                Some(matches) => issues.push(DependencyIssue {
                    kind: DependencyIssueKind::AmbiguousHardDependency,
                    artifact_path: artifact.path.clone(),
                    artifact_id: artifact.id.clone(),
                    dependency: dependency.clone(),
                    candidates: matches.iter().map(|item| item.path.clone()).collect(),
                }),
                None => {
                    let base = archetype_base_id(dependency);
                    let candidates: Vec<String> = archetypes
                        .keys()
                        .filter(|candidate| archetype_base_id(candidate) == base)
                        .map(|candidate| (*candidate).to_string())
                        .collect();
                    issues.push(DependencyIssue {
                        kind: if candidates.is_empty() {
                            DependencyIssueKind::MissingHardDependency
                        } else {
                            DependencyIssueKind::MajorVersionMismatch
                        },
                        artifact_path: artifact.path.clone(),
                        artifact_id: artifact.id.clone(),
                        dependency: dependency.clone(),
                        candidates,
                    });
                }
            }
        }
    }

    issues.sort_by(|a, b| {
        (&a.artifact_path, a.kind, &a.dependency).cmp(&(&b.artifact_path, b.kind, &b.dependency))
    });
    issues
}

fn summarize(artifacts: &[Artifact], issues: &[DependencyIssue]) -> InventorySummary {
    let mut summary = InventorySummary {
        artifacts: artifacts.len(),
        ..InventorySummary::default()
    };
    for artifact in artifacts {
        match artifact.kind {
            ArtifactKind::Archetype => {
                summary.archetypes += 1;
                if artifact.lifecycle_state.as_deref() == Some("published") {
                    summary.published_archetypes += 1;
                }
            }
            ArtifactKind::Template => summary.templates += 1,
            ArtifactKind::Termset => summary.termsets += 1,
        }
        let lifecycle = artifact.lifecycle_state.as_deref().unwrap_or("missing");
        *summary
            .lifecycle_states
            .entry(lifecycle.to_string())
            .or_default() += 1;
        let license = artifact
            .license_spdx
            .as_deref()
            .unwrap_or("missing_or_other");
        *summary.licenses.entry(license.to_string()).or_default() += 1;
        for issue in &artifact.parse_issues {
            *summary.parse_issues.entry(issue.clone()).or_default() += 1;
        }
    }
    for issue in issues {
        *summary
            .dependency_issues
            .entry(issue.kind.as_str().to_string())
            .or_default() += 1;
    }
    summary
}

fn source_evidence(root: &Path) -> Result<SourceEvidence, InventoryError> {
    let license_path = root.join("LICENSE");
    let (repository_license, repository_license_sha256) = if license_path.is_file() {
        let bytes = fs::read(&license_path).map_err(|source| InventoryError::Io {
            path: license_path,
            source,
        })?;
        (
            Some(String::from_utf8_lossy(&bytes).trim().to_string()),
            Some(digest(&bytes)),
        )
    } else {
        (None, None)
    };
    let has_git = root.join(".git").exists();
    let git_revision = if has_git {
        Command::new("git")
            .args(["-C"])
            .arg(root)
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|revision| revision.trim().to_string())
    } else {
        None
    };
    let git_dirty = if has_git {
        Command::new("git")
            .args(["-C"])
            .arg(root)
            .args(["status", "--porcelain"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| !output.stdout.is_empty())
    } else {
        None
    };
    Ok(SourceEvidence {
        git_revision,
        git_dirty,
        repository_license,
        repository_license_sha256,
    })
}

fn artifact_origin(relative: &str) -> String {
    let mut components = relative.split('/');
    match (components.next(), components.next()) {
        (Some("remote"), Some(repository)) => format!("remote/{repository}"),
        (Some(origin), _) => origin.to_string(),
        _ => "unknown".to_string(),
    }
}

fn extract_adl_dependencies(text: &str) -> Vec<String> {
    let mut dependencies = BTreeSet::new();
    let mut expect_parent = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "specialize" {
            expect_parent = true;
            continue;
        }
        if expect_parent && !trimmed.is_empty() {
            if trimmed.starts_with("openEHR-") {
                dependencies.insert(trimmed.to_string());
            }
            expect_parent = false;
        }
        if trimmed.contains("use_archetype") {
            if let Some(id) = extract_openehr_id(trimmed) {
                dependencies.insert(id);
            }
        }
    }
    dependencies.into_iter().collect()
}

fn extract_slot_constraints(text: &str) -> Vec<String> {
    let mut constraints = BTreeSet::new();
    for line in text.lines().map(str::trim) {
        if !line.contains("archetype_id/value matches") {
            continue;
        }
        if let (Some(start), Some(end)) = (line.find('{'), line.rfind('}')) {
            if start < end {
                constraints.insert(line[start + 1..end].trim().to_string());
            }
        }
    }
    constraints.into_iter().collect()
}

fn extract_languages(text: &str) -> Vec<String> {
    let mut languages = BTreeSet::new();
    let marker = "[ISO_639-1::";
    let mut remaining = text;
    while let Some(start) = remaining.find(marker) {
        let value = &remaining[start + marker.len()..];
        if let Some(end) = value.find(']') {
            languages.insert(value[..end].to_string());
            remaining = &value[end + 1..];
        } else {
            break;
        }
    }
    languages.into_iter().collect()
}

fn extract_odin_scalar(text: &str, key: &str) -> Option<String> {
    let marker = format!("{key} = <\"");
    text.find(&marker).and_then(|start| {
        let value = &text[start + marker.len()..];
        value.find("\">").map(|end| value[..end].to_string())
    })
}

fn extract_odin_map_value(text: &str, key: &str) -> Option<String> {
    let marker = format!("[\"{key}\"] = <\"");
    text.find(&marker).and_then(|start| {
        let value = &text[start + marker.len()..];
        value.find("\">").map(|end| value[..end].to_string())
    })
}

fn extract_between(line: &str, marker: &str, terminators: &[char]) -> Option<String> {
    let start = line.find(marker)? + marker.len();
    let value = &line[start..];
    let end = value.find(terminators).unwrap_or(value.len());
    Some(value[..end].trim().to_string())
}

fn extract_openehr_id(line: &str) -> Option<String> {
    let start = line.find("openEHR-")?;
    let value = &line[start..];
    let end = value
        .find(|character: char| character == ']' || character.is_whitespace())
        .unwrap_or(value.len());
    Some(value[..end].trim_end_matches(',').to_string())
}

fn extract_xml_element(text: &str, element: &str) -> Option<String> {
    let opening = format!("<{element}>");
    let closing = format!("</{element}>");
    let start = text.find(&opening)? + opening.len();
    let value = &text[start..];
    let end = value.find(&closing)?;
    Some(xml_unescape(value[..end].trim()))
}

fn extract_xml_map_value(text: &str, key: &str) -> Option<String> {
    let marker = format!("<key>{key}</key>");
    let start = text.find(&marker)? + marker.len();
    let remaining = &text[start..];
    let value_start = remaining.find("<value")?;
    let value = &remaining[value_start..];
    if value.starts_with("<value/>") || value.starts_with("<value />") {
        return Some(String::new());
    }
    let content_start = value.find('>')? + 1;
    let content = &value[content_start..];
    let end = content.find("</value>")?;
    Some(xml_unescape(content[..end].trim()))
}

fn extract_xml_attributes(text: &str, attribute: &str) -> Vec<String> {
    let mut values = BTreeSet::new();
    let marker = format!("{attribute}=\"");
    let mut remaining = text;
    while let Some(start) = remaining.find(&marker) {
        let value = &remaining[start + marker.len()..];
        if let Some(end) = value.find('"') {
            values.insert(xml_unescape(&value[..end]));
            remaining = &value[end + 1..];
        } else {
            break;
        }
    }
    values.into_iter().collect()
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn normalize_license(value: &str) -> Option<String> {
    let lowercase = value.to_ascii_lowercase();
    if lowercase.contains("by-sa/4.0") || lowercase.contains("sharealike 4.0") {
        Some("CC-BY-SA-4.0".to_string())
    } else if lowercase.contains("by-sa/3.0") || lowercase.contains("sharealike 3.0") {
        Some("CC-BY-SA-3.0".to_string())
    } else {
        None
    }
}

fn rm_type(id: &str) -> Option<String> {
    id.split('-').nth(2)?.split('.').next().map(str::to_string)
}

fn archetype_base_id(id: &str) -> &str {
    id.rsplit_once(".v")
        .filter(|(_, version)| version.chars().all(|character| character.is_ascii_digit()))
        .map(|(base, _)| base)
        .unwrap_or(id)
}

fn digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn relative_path(root: &Path, path: &Path) -> Result<String, InventoryError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| InventoryError::NonUtf8Path(path.to_path_buf()))?;
    let value = relative
        .to_str()
        .ok_or_else(|| InventoryError::NonUtf8Path(relative.to_path_buf()))?;
    Ok(value.replace('\\', "/"))
}
