// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Policy resolution and deterministic locking for inventoried knowledge.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{Artifact, ArtifactKind, Inventory};

const MANIFEST_VERSION: u32 = 1;
const LOCK_VERSION: u32 = 1;

/// Human-authored source selection and acceptance policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeManifest {
    #[serde(default = "manifest_version")]
    pub version: u32,
    pub knowledge: KnowledgeDeclaration,
    #[serde(default)]
    pub source: SourcePolicy,
    #[serde(default)]
    pub policy: ResolutionPolicy,
    #[serde(default)]
    pub languages: LanguagePolicy,
    #[serde(default)]
    pub artefacts: ArtefactSelection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeDeclaration {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SourcePolicy {
    pub origins: Vec<String>,
    #[serde(rename = "allow-dirty-checkout")]
    pub allow_dirty_checkout: bool,
}

impl Default for SourcePolicy {
    fn default() -> Self {
        Self {
            origins: vec!["local".to_string()],
            allow_dirty_checkout: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResolutionPolicy {
    #[serde(rename = "allowed-lifecycle-states")]
    pub allowed_lifecycle_states: Vec<String>,
    #[serde(rename = "allowed-licences")]
    pub allowed_licenses: Vec<String>,
    #[serde(rename = "allow-missing-hard-dependencies")]
    pub allow_missing_hard_dependencies: bool,
}

impl Default for ResolutionPolicy {
    fn default() -> Self {
        Self {
            allowed_lifecycle_states: vec!["published".to_string()],
            allowed_licenses: vec!["CC-BY-SA-3.0".to_string(), "CC-BY-SA-4.0".to_string()],
            allow_missing_hard_dependencies: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LanguagePolicy {
    pub include: Vec<String>,
}

impl Default for LanguagePolicy {
    fn default() -> Self {
        Self {
            include: vec!["en".to_string()],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArtefactSelection {
    pub include: Vec<String>,
}

/// Complete deterministic resolution, including explanations for exclusions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub lock: KnowledgeLock,
    pub decisions: Vec<ResolutionDecision>,
    pub issues: Vec<ResolutionIssue>,
}

impl Resolution {
    pub fn is_success(&self) -> bool {
        !self.issues.iter().any(|issue| !issue.allowed)
    }

    pub fn blockers(&self) -> impl Iterator<Item = &ResolutionIssue> {
        self.issues.iter().filter(|issue| !issue.allowed)
    }

    pub fn explain(&self, artefact_id: &str) -> Vec<&ResolutionDecision> {
        self.decisions
            .iter()
            .filter(|decision| decision.id.as_deref() == Some(artefact_id))
            .collect()
    }
}

/// Generated authoritative source resolution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeLock {
    pub version: u32,
    pub knowledge: LockedKnowledge,
    pub source: LockedSource,
    #[serde(rename = "artefact", default)]
    pub artefacts: Vec<LockedArtefact>,
}

impl KnowledgeLock {
    pub fn from_path(path: &Path) -> Result<Self, ResolutionError> {
        let content = fs::read_to_string(path).map_err(|source| ResolutionError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str(&content).map_err(|source| ResolutionError::ParseLock {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn to_toml(&self) -> Result<String, ResolutionError> {
        let mut rendered = toml::to_string_pretty(self).map_err(ResolutionError::SerializeLock)?;
        if !rendered.ends_with('\n') {
            rendered.push('\n');
        }
        Ok(rendered)
    }

    pub fn write_atomic(&self, path: &Path) -> Result<(), ResolutionError> {
        let rendered = self.to_toml()?;
        let temporary = temporary_path(path);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|source| ResolutionError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.write_all(rendered.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|source| ResolutionError::Io {
                path: temporary.clone(),
                source,
            })?;
        replace_file(&temporary, path).map_err(|source| ResolutionError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedKnowledge {
    pub name: String,
    #[serde(rename = "manifest-sha256", default)]
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    pub dirty: bool,
    #[serde(rename = "inventory-sha256")]
    pub inventory_sha256: String,
    pub origins: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedArtefact {
    pub id: String,
    pub kind: ArtifactKind,
    pub path: String,
    pub origin: String,
    pub checksum: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub licence: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    #[serde(
        rename = "slot-constraints",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub slot_constraints: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionDecision {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub included: bool,
    pub reason: DecisionReason,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionReason {
    SelectedByPolicy,
    ExplicitlySelected,
    HardDependency,
    NotSelected,
    OriginNotAllowed,
    LifecycleNotAllowed,
    LicenceNotAllowed,
    LanguageNotIncluded,
    MissingIdentifier,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionIssue {
    pub kind: ResolutionIssueKind,
    pub artefact_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artefact_id: Option<String>,
    pub dependency: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    pub allowed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionIssueKind {
    RequestedArtefactMissing,
    DuplicateArtefactId,
    MissingHardDependency,
    MajorVersionMismatch,
    AmbiguousHardDependency,
    PolicyExcludedHardDependency,
}

#[derive(Debug, Error)]
pub enum ResolutionError {
    #[error("reading {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("parsing knowledge manifest {path}: {source}")]
    ParseManifest {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("parsing knowledge lock {path}: {source}")]
    ParseLock {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("unsupported knowledge manifest version {0}; expected {MANIFEST_VERSION}")]
    UnsupportedManifestVersion(u32),
    #[error("knowledge manifest name must not be empty")]
    EmptyKnowledgeName,
    #[error("source checkout is dirty; commit or stash changes, or set source.allow-dirty-checkout = true")]
    DirtyCheckout,
    #[error("serializing knowledge lock: {0}")]
    SerializeLock(toml::ser::Error),
}

impl KnowledgeManifest {
    /// A minimal deployment manifest with the safe International-source policy.
    pub fn deployment_default() -> Self {
        Self {
            version: MANIFEST_VERSION,
            knowledge: KnowledgeDeclaration {
                name: "anarchie-default".to_string(),
            },
            source: SourcePolicy::default(),
            policy: ResolutionPolicy::default(),
            languages: LanguagePolicy::default(),
            artefacts: ArtefactSelection::default(),
        }
    }

    pub fn to_toml(&self) -> Result<String, ResolutionError> {
        let mut rendered = toml::to_string_pretty(self).map_err(ResolutionError::SerializeLock)?;
        if !rendered.ends_with('\n') {
            rendered.push('\n');
        }
        Ok(rendered)
    }

    pub fn digest(&self) -> Result<String, ResolutionError> {
        Ok(digest(self.to_toml()?.as_bytes()))
    }

    pub fn from_path(path: &Path) -> Result<Self, ResolutionError> {
        let content = fs::read_to_string(path).map_err(|source| ResolutionError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let manifest: Self =
            toml::from_str(&content).map_err(|source| ResolutionError::ParseManifest {
                path: path.to_path_buf(),
                source,
            })?;
        if manifest.version != MANIFEST_VERSION {
            return Err(ResolutionError::UnsupportedManifestVersion(
                manifest.version,
            ));
        }
        if manifest.knowledge.name.trim().is_empty() {
            return Err(ResolutionError::EmptyKnowledgeName);
        }
        Ok(manifest)
    }

    pub fn resolve(&self, inventory: &Inventory) -> Result<Resolution, ResolutionError> {
        if inventory.source.git_dirty == Some(true) && !self.source.allow_dirty_checkout {
            return Err(ResolutionError::DirtyCheckout);
        }

        let policy_reasons: Vec<DecisionReason> = inventory
            .artifacts
            .iter()
            .map(|artefact| self.policy_reason(artefact))
            .collect();
        let mut selected = BTreeMap::<usize, DecisionReason>::new();
        let mut issues = Vec::new();

        if self.artefacts.include.is_empty() {
            for (index, reason) in policy_reasons.iter().enumerate() {
                if *reason == DecisionReason::SelectedByPolicy {
                    selected.insert(index, DecisionReason::SelectedByPolicy);
                }
            }
            let mut selected_ids = BTreeMap::<&str, Vec<usize>>::new();
            for index in selected.keys() {
                if let Some(id) = inventory.artifacts[*index].id.as_deref() {
                    selected_ids.entry(id).or_default().push(*index);
                }
            }
            for (id, candidates) in selected_ids {
                if candidates.len() > 1 {
                    issues.push(issue(
                        ResolutionIssueKind::DuplicateArtefactId,
                        "",
                        Some(id.to_string()),
                        id,
                        candidates
                            .iter()
                            .map(|index| inventory.artifacts[*index].path.clone())
                            .collect(),
                        false,
                    ));
                }
            }
        } else {
            for requested in sorted_unique(&self.artefacts.include) {
                let candidates = matching_ids(inventory, &requested, &self.source.origins);
                match candidates.as_slice() {
                    [] => issues.push(issue(
                        ResolutionIssueKind::RequestedArtefactMissing,
                        "",
                        None,
                        &requested,
                        Vec::new(),
                        false,
                    )),
                    [index] if policy_reasons[*index] == DecisionReason::SelectedByPolicy => {
                        selected.insert(*index, DecisionReason::ExplicitlySelected);
                    }
                    [index] => issues.push(issue(
                        ResolutionIssueKind::PolicyExcludedHardDependency,
                        &inventory.artifacts[*index].path,
                        inventory.artifacts[*index].id.clone(),
                        &requested,
                        vec![inventory.artifacts[*index].path.clone()],
                        false,
                    )),
                    _ => issues.push(issue(
                        ResolutionIssueKind::DuplicateArtefactId,
                        "",
                        Some(requested.clone()),
                        &requested,
                        candidates
                            .iter()
                            .map(|index| inventory.artifacts[*index].path.clone())
                            .collect(),
                        false,
                    )),
                }
            }
        }

        let mut queue: VecDeque<usize> = selected.keys().copied().collect();
        while let Some(requester) = queue.pop_front() {
            for dependency in &inventory.artifacts[requester].hard_dependencies {
                let all_candidates = matching_archetype_ids(inventory, dependency);
                let candidates: Vec<usize> = all_candidates
                    .iter()
                    .copied()
                    .filter(|index| {
                        origin_allowed(&inventory.artifacts[*index].origin, &self.source.origins)
                    })
                    .collect();
                match candidates.as_slice() {
                    [index] if policy_reasons[*index] == DecisionReason::SelectedByPolicy => {
                        if !selected.contains_key(index) {
                            selected.insert(*index, DecisionReason::HardDependency);
                            queue.push_back(*index);
                        }
                    }
                    [index] => issues.push(issue(
                        ResolutionIssueKind::PolicyExcludedHardDependency,
                        &inventory.artifacts[requester].path,
                        inventory.artifacts[requester].id.clone(),
                        dependency,
                        vec![inventory.artifacts[*index].path.clone()],
                        false,
                    )),
                    [] => {
                        if !all_candidates.is_empty() {
                            issues.push(issue(
                                ResolutionIssueKind::PolicyExcludedHardDependency,
                                &inventory.artifacts[requester].path,
                                inventory.artifacts[requester].id.clone(),
                                dependency,
                                all_candidates
                                    .iter()
                                    .map(|index| inventory.artifacts[*index].path.clone())
                                    .collect(),
                                false,
                            ));
                            continue;
                        }
                        let base = archetype_base_id(dependency);
                        let major_candidates: Vec<String> = inventory
                            .artifacts
                            .iter()
                            .filter(|candidate| candidate.kind == ArtifactKind::Archetype)
                            .filter(|candidate| {
                                origin_allowed(&candidate.origin, &self.source.origins)
                            })
                            .filter_map(|candidate| candidate.id.as_deref())
                            .filter(|candidate| archetype_base_id(candidate) == base)
                            .map(str::to_string)
                            .collect();
                        let kind = if major_candidates.is_empty() {
                            ResolutionIssueKind::MissingHardDependency
                        } else {
                            ResolutionIssueKind::MajorVersionMismatch
                        };
                        issues.push(issue(
                            kind,
                            &inventory.artifacts[requester].path,
                            inventory.artifacts[requester].id.clone(),
                            dependency,
                            major_candidates,
                            kind == ResolutionIssueKind::MissingHardDependency
                                && self.policy.allow_missing_hard_dependencies,
                        ));
                    }
                    _ => issues.push(issue(
                        ResolutionIssueKind::AmbiguousHardDependency,
                        &inventory.artifacts[requester].path,
                        inventory.artifacts[requester].id.clone(),
                        dependency,
                        candidates
                            .iter()
                            .map(|index| inventory.artifacts[*index].path.clone())
                            .collect(),
                        false,
                    )),
                }
            }
        }

        issues.sort_by(|a, b| {
            (&a.artefact_path, a.kind, &a.dependency).cmp(&(
                &b.artefact_path,
                b.kind,
                &b.dependency,
            ))
        });
        issues.dedup();

        let decisions = inventory
            .artifacts
            .iter()
            .enumerate()
            .map(|(index, artefact)| ResolutionDecision {
                path: artefact.path.clone(),
                id: artefact.id.clone(),
                included: selected.contains_key(&index),
                reason: selected.get(&index).cloned().unwrap_or_else(|| {
                    if policy_reasons[index] == DecisionReason::SelectedByPolicy
                        && !self.artefacts.include.is_empty()
                    {
                        DecisionReason::NotSelected
                    } else {
                        policy_reasons[index].clone()
                    }
                }),
            })
            .collect();
        let artefacts = selected
            .keys()
            .map(|index| lock_artefact(&inventory.artifacts[*index]))
            .collect();
        let lock = KnowledgeLock {
            version: LOCK_VERSION,
            knowledge: LockedKnowledge {
                name: self.knowledge.name.clone(),
                manifest_sha256: self.digest()?,
            },
            source: LockedSource {
                revision: inventory.source.git_revision.clone(),
                dirty: inventory.source.git_dirty.unwrap_or(false),
                inventory_sha256: inventory_digest(inventory),
                origins: sorted_unique(&self.source.origins),
            },
            artefacts,
        };

        Ok(Resolution {
            lock,
            decisions,
            issues,
        })
    }

    fn policy_reason(&self, artefact: &Artifact) -> DecisionReason {
        if artefact.id.is_none() {
            return DecisionReason::MissingIdentifier;
        }
        if !origin_allowed(&artefact.origin, &self.source.origins) {
            return DecisionReason::OriginNotAllowed;
        }
        if !self.policy.allowed_lifecycle_states.is_empty()
            && !artefact
                .lifecycle_state
                .as_ref()
                .is_some_and(|state| self.policy.allowed_lifecycle_states.contains(state))
        {
            return DecisionReason::LifecycleNotAllowed;
        }
        if !self.policy.allowed_licenses.is_empty()
            && !artefact
                .license_spdx
                .as_ref()
                .is_some_and(|license| self.policy.allowed_licenses.contains(license))
        {
            return DecisionReason::LicenceNotAllowed;
        }
        if !self.languages.include.is_empty()
            && !artefact.languages.is_empty()
            && !artefact
                .languages
                .iter()
                .any(|language| self.languages.include.contains(language))
        {
            return DecisionReason::LanguageNotIncluded;
        }
        DecisionReason::SelectedByPolicy
    }
}

/// Compact comparison of the manifest, source inventory, and an optional lock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KnowledgeStatus {
    pub state: KnowledgeStatusState,
    pub knowledge_name: String,
    pub manifest_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_manifest_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_source_revision: Option<String>,
    pub inventory_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_inventory_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_artefacts: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeStatusState {
    Current,
    Stale,
    Unresolved,
}

impl KnowledgeStatus {
    pub fn inspect(
        manifest: &KnowledgeManifest,
        lock: Option<&KnowledgeLock>,
        inventory: &Inventory,
    ) -> Result<Self, ResolutionError> {
        let manifest_sha256 = manifest.digest()?;
        let inventory_sha256 = inventory_digest(inventory);
        let state = match lock {
            None => KnowledgeStatusState::Unresolved,
            Some(lock)
                if lock.knowledge.manifest_sha256 == manifest_sha256
                    && lock.source.revision == inventory.source.git_revision
                    && lock.source.inventory_sha256 == inventory_sha256 =>
            {
                KnowledgeStatusState::Current
            }
            Some(_) => KnowledgeStatusState::Stale,
        };
        Ok(Self {
            state,
            knowledge_name: manifest.knowledge.name.clone(),
            manifest_sha256,
            lock_manifest_sha256: lock.map(|lock| lock.knowledge.manifest_sha256.clone()),
            source_revision: inventory.source.git_revision.clone(),
            lock_source_revision: lock.and_then(|lock| lock.source.revision.clone()),
            inventory_sha256,
            lock_inventory_sha256: lock.map(|lock| lock.source.inventory_sha256.clone()),
            selected_artefacts: lock.map(|lock| lock.artefacts.len()),
        })
    }
}

fn lock_artefact(artefact: &Artifact) -> LockedArtefact {
    LockedArtefact {
        id: artefact
            .id
            .clone()
            .expect("selected artefacts have identifiers"),
        kind: artefact.kind,
        path: artefact.path.clone(),
        origin: artefact.origin.clone(),
        checksum: format!("sha256:{}", artefact.sha256),
        revision: artefact.revision.clone(),
        lifecycle: artefact.lifecycle_state.clone(),
        licence: artefact.license_spdx.clone(),
        languages: artefact.languages.clone(),
        dependencies: artefact.hard_dependencies.clone(),
        slot_constraints: artefact.slot_constraints.clone(),
    }
}

fn issue(
    kind: ResolutionIssueKind,
    path: &str,
    artefact_id: Option<String>,
    dependency: &str,
    candidates: Vec<String>,
    allowed: bool,
) -> ResolutionIssue {
    ResolutionIssue {
        kind,
        artefact_path: path.to_string(),
        artefact_id,
        dependency: dependency.to_string(),
        candidates,
        allowed,
    }
}

fn matching_ids(inventory: &Inventory, id: &str, origins: &[String]) -> Vec<usize> {
    inventory
        .artifacts
        .iter()
        .enumerate()
        .filter(|(_, artefact)| origin_allowed(&artefact.origin, origins))
        .filter(|(_, artefact)| artefact.id.as_deref() == Some(id))
        .map(|(index, _)| index)
        .collect()
}

fn matching_archetype_ids(inventory: &Inventory, id: &str) -> Vec<usize> {
    inventory
        .artifacts
        .iter()
        .enumerate()
        .filter(|(_, artefact)| artefact.kind == ArtifactKind::Archetype)
        .filter(|(_, artefact)| artefact.id.as_deref() == Some(id))
        .map(|(index, _)| index)
        .collect()
}

fn origin_allowed(origin: &str, allowed: &[String]) -> bool {
    allowed.is_empty()
        || allowed.iter().any(|candidate| {
            origin == candidate || (candidate == "remote" && origin.starts_with("remote/"))
        })
}

fn sorted_unique(values: &[String]) -> Vec<String> {
    values
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn archetype_base_id(id: &str) -> &str {
    id.rsplit_once(".v")
        .filter(|(_, version)| version.chars().all(|character| character.is_ascii_digit()))
        .map(|(base, _)| base)
        .unwrap_or(id)
}

fn inventory_digest(inventory: &Inventory) -> String {
    let mut hasher = Sha256::new();
    for artefact in &inventory.artifacts {
        hasher.update(artefact.path.as_bytes());
        hasher.update([0]);
        hasher.update(artefact.sha256.as_bytes());
        hasher.update(b"\n");
    }
    digest_bytes(hasher.finalize())
}

impl Inventory {
    /// SHA-256 over stable relative paths and content digests in inventory order.
    pub fn content_digest(&self) -> String {
        inventory_digest(self)
    }
}

fn digest(bytes: &[u8]) -> String {
    digest_bytes(Sha256::digest(bytes))
}

fn digest_bytes(bytes: impl IntoIterator<Item = u8>) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(64);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("knowledge.lock");
    path.with_file_name(format!(".{name}.{}.tmp", std::process::id()))
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.exists() {
        fs::remove_file(to)?;
    }
    fs::rename(from, to)
}

const fn manifest_version() -> u32 {
    MANIFEST_VERSION
}
