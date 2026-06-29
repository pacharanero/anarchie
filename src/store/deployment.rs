// SPDX-License-Identifier: AGPL-3.0-or-later
//! The deployment and per-EHR repository operations.
//!
//! A *deployment* is a directory containing `anarchie.toml`, shared
//! `templates/`, a git-ignored derived `index/`, and an `ehrs/` directory of
//! one-git-repository-per-EHR. See `specs/on-disk-format.md`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::rm::{Composition, Ehr, EhrStatus, UidBasedId};
use crate::validate::Opt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::store::config::DeploymentConfig;
use crate::store::error::{Result, StoreError};
use crate::store::git::Git;

/// The kind of change a contribution represents, mirroring openEHR's
/// `AUDIT_DETAILS.change_type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeType {
    Creation,
    Modification,
    Deletion,
}

impl ChangeType {
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeType::Creation => "creation",
            ChangeType::Modification => "modification",
            ChangeType::Deletion => "deletion",
        }
    }
}

/// The audit data for one contribution: who, when, why, and what kind.
#[derive(Clone, Debug)]
pub struct Audit {
    pub committer_name: String,
    pub committer_email: String,
    /// RFC 3339 / ISO 8601 timestamp, e.g. `2026-06-18T10:14:22Z`.
    pub time_committed: String,
    pub change_type: ChangeType,
    pub description: String,
}

impl Audit {
    /// A creation contribution stamped at the current instant.
    pub fn now(
        committer_name: impl Into<String>,
        committer_email: impl Into<String>,
        change_type: ChangeType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            committer_name: committer_name.into(),
            committer_email: committer_email.into(),
            time_committed: now_iso8601(),
            change_type,
            description: description.into(),
        }
    }
}

/// Current UTC time as a second-precision RFC 3339 string.
pub fn now_iso8601() -> String {
    jiff::Timestamp::now()
        .round(jiff::Unit::Second)
        .unwrap_or_else(|_| jiff::Timestamp::now())
        .to_string()
}

/// The denormalised contribution manifest written alongside the commit, so the
/// audit trail is readable without invoking git.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributionManifest {
    #[serde(rename = "_type")]
    pub ty: String,
    pub uid: ManifestUid,
    /// The commit that carries this contribution. Omitted in the file (a commit
    /// cannot contain its own hash); the contribution-to-commit link is the
    /// `anarchie-contribution-id` commit trailer. Populated when read back.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub commit: Option<String>,
    pub versions: Vec<String>,
    pub audit: ManifestAudit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestUid {
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestAudit {
    pub committer: ManifestCommitter,
    pub time_committed: String,
    pub change_type: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestCommitter {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub email: Option<String>,
}

/// The outcome of committing one composition.
#[derive(Clone, Debug)]
pub struct CommitOutcome {
    pub object_id: String,
    pub version_uid: String,
    pub commit_sha: String,
    pub contribution_id: String,
}

/// A handle to an anarchie deployment.
#[derive(Clone, Debug)]
pub struct Deployment {
    root: PathBuf,
    config: DeploymentConfig,
}

impl Deployment {
    /// Scaffold a new deployment at `root`, writing `anarchie.toml` and the
    /// standard directory skeleton. Fails if `anarchie.toml` already exists.
    pub fn init(root: impl Into<PathBuf>, config: DeploymentConfig) -> Result<Self> {
        let root = root.into();
        let config_path = root.join("anarchie.toml");
        if config_path.exists() {
            return Err(StoreError::AlreadyExists(root));
        }

        create_dir(&root)?;
        create_dir(&root.join("templates"))?;
        create_dir(&root.join("ehrs"))?;
        create_dir(&root.join("index"))?;

        let toml = toml::to_string_pretty(&config)?;
        write_file(&config_path, &toml)?;
        write_file(
            &root.join("templates").join("index.json"),
            "{\n  \"templates\": []\n}\n",
        )?;
        // The derived index is never authoritative and is not committed.
        write_file(&root.join("index").join(".gitignore"), "*\n")?;

        Ok(Self { root, config })
    }

    /// Open the deployment containing `start`, walking up to find
    /// `anarchie.toml`.
    pub fn open(start: impl AsRef<Path>) -> Result<Self> {
        let mut dir = start.as_ref().to_path_buf();
        loop {
            let candidate = dir.join("anarchie.toml");
            if candidate.exists() {
                let text = fs::read_to_string(&candidate).map_err(|source| StoreError::Io {
                    path: candidate.clone(),
                    source,
                })?;
                let config: DeploymentConfig = toml::from_str(&text)?;
                return Ok(Self { root: dir, config });
            }
            if !dir.pop() {
                return Err(StoreError::NotADeployment(start.as_ref().to_path_buf()));
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config(&self) -> &DeploymentConfig {
        &self.config
    }

    pub fn ehrs_dir(&self) -> PathBuf {
        self.root.join("ehrs")
    }

    /// Create a fresh EHR repository and return a handle to it.
    pub fn create_ehr(&self, audit: &Audit) -> Result<EhrRepo> {
        let ehr_id = Uuid::new_v4().to_string();
        let path = self.ehrs_dir().join(&ehr_id);
        create_dir(&path)?;

        let git = Git::open(&path);
        git.init()?;

        let ehr = Ehr::new(
            ehr_id.clone(),
            self.config.system_id.clone(),
            audit.time_committed.clone(),
        );
        write_file(
            &path.join("ehr.json"),
            &crate::rm::to_canonical_string(&ehr)?,
        )?;

        create_dir(&path.join("ehr_status"))?;
        let status = EhrStatus::default_for("openEHR-EHR-EHR_STATUS.generic.v1");
        write_file(
            &path.join("ehr_status").join("status.json"),
            &crate::rm::to_canonical_string(&status)?,
        )?;

        git.add("ehr.json")?;
        git.add("ehr_status/status.json")?;
        git.commit(
            &format!("Create EHR {ehr_id}"),
            &audit.committer_name,
            &audit.committer_email,
            &audit.time_committed,
            &[
                ("anarchie-change-type", "creation"),
                ("anarchie-system-id", &self.config.system_id),
            ],
        )?;

        Ok(EhrRepo {
            ehr_id,
            system_id: self.config.system_id.clone(),
            path,
            git,
            templates_dir: self.templates_dir(),
        })
    }

    /// List the EHR ids present in the deployment.
    pub fn list_ehrs(&self) -> Result<Vec<String>> {
        let dir = self.ehrs_dir();
        let mut ids = Vec::new();
        if !dir.exists() {
            return Ok(ids);
        }
        let entries = fs::read_dir(&dir).map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| StoreError::Io {
                path: dir.clone(),
                source,
            })?;
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    ids.push(name.to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Open an existing EHR repository by id.
    pub fn open_ehr(&self, ehr_id: &str) -> Result<EhrRepo> {
        let path = self.ehrs_dir().join(ehr_id);
        if !path.join("ehr.json").exists() {
            return Err(StoreError::EhrNotFound(ehr_id.to_string()));
        }
        Ok(EhrRepo {
            ehr_id: ehr_id.to_string(),
            system_id: self.config.system_id.clone(),
            path: path.clone(),
            git: Git::open(path),
            templates_dir: self.templates_dir(),
        })
    }
}

/// A handle to one EHR's git repository.
#[derive(Clone, Debug)]
pub struct EhrRepo {
    ehr_id: String,
    system_id: String,
    path: PathBuf,
    git: Git,
    templates_dir: PathBuf,
}

impl EhrRepo {
    pub fn ehr_id(&self) -> &str {
        &self.ehr_id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn composition_rel(object_id: &str) -> String {
        format!("compositions/{object_id}/composition.json")
    }

    /// Load a registered Operational Template from the deployment's shared
    /// `templates/` directory, if one is registered under `template_id`.
    fn load_template(&self, template_id: &str) -> Result<Option<Opt>> {
        let path = self.templates_dir.join(format!("{template_id}.opt.json"));
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&path).map_err(|source| StoreError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(Some(Opt::from_json(&json)?))
    }

    /// Commit a composition as a new version, validating it first. If
    /// `object_id` is `Some`, this is a modification of that versioned object;
    /// otherwise a new object is created. The composition is checked against the
    /// Reference Model (always) and its claimed Operational Template (if one is
    /// registered); a composition with validation errors is rejected and
    /// nothing is written. Returns the assigned `version_uid` and commit
    /// metadata.
    pub fn commit_composition(
        &self,
        composition: Composition,
        object_id: Option<String>,
        audit: &Audit,
    ) -> Result<CommitOutcome> {
        self.commit_composition_with(composition, object_id, audit, true)
    }

    /// Commit a composition *without* validating it. For tooling that has
    /// already validated, or for deliberately storing known-nonconformant data.
    pub fn commit_composition_unchecked(
        &self,
        composition: Composition,
        object_id: Option<String>,
        audit: &Audit,
    ) -> Result<CommitOutcome> {
        self.commit_composition_with(composition, object_id, audit, false)
    }

    fn commit_composition_with(
        &self,
        mut composition: Composition,
        object_id: Option<String>,
        audit: &Audit,
        validate: bool,
    ) -> Result<CommitOutcome> {
        if validate {
            let template = match composition.archetype_details.template_id.as_ref() {
                Some(template_id) => self.load_template(&template_id.value)?,
                None => None,
            };
            let report = crate::validate::validate(&composition, template.as_ref());
            if report.error_count() > 0 {
                return Err(StoreError::Invalid(report));
            }
        }

        let object_id = object_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let rel = Self::composition_rel(&object_id);
        let abs = self.path.join(&rel);

        let prior_versions = self.git.commit_count(&rel)?;
        let version_tree_id = prior_versions + 1;
        let version_uid = format!("{object_id}::{}::{version_tree_id}", self.system_id);

        composition.uid = Some(UidBasedId::ObjectVersionId {
            value: version_uid.clone(),
        });

        if let Some(parent) = abs.parent() {
            create_dir(parent)?;
        }
        write_file(&abs, &crate::rm::to_canonical_string(&composition)?)?;

        // The commit subject is the audit description - openEHR's
        // AUDIT_DETAILS.description - so `git log` and `anarchie log` read in the
        // committer's own words. With no description supplied we fall back to a
        // generated summary that still records create-vs-update and the version.
        let description = if audit.description.trim().is_empty() {
            if version_tree_id == 1 {
                format!("Create composition {object_id}")
            } else {
                format!("Update composition {object_id} (v{version_tree_id})")
            }
        } else {
            audit.description.clone()
        };

        let contribution_id = Uuid::new_v4().to_string();
        let contrib_rel = format!("contributions/{contribution_id}-contrib.json");
        let contrib_abs = self.path.join(&contrib_rel);
        if let Some(parent) = contrib_abs.parent() {
            create_dir(parent)?;
        }

        let manifest = ContributionManifest {
            ty: "CONTRIBUTION".to_string(),
            uid: ManifestUid {
                value: contribution_id.clone(),
            },
            commit: None,
            versions: vec![version_uid.clone()],
            audit: ManifestAudit {
                committer: ManifestCommitter {
                    name: audit.committer_name.clone(),
                    email: Some(audit.committer_email.clone()),
                },
                time_committed: audit.time_committed.clone(),
                change_type: audit.change_type.as_str().to_string(),
                description: description.clone(),
            },
        };
        write_file(&contrib_abs, &to_pretty_json(&manifest)?)?;

        self.git.add(&rel)?;
        self.git.add(&contrib_rel)?;
        let sha = self.git.commit(
            &description,
            &audit.committer_name,
            &audit.committer_email,
            &audit.time_committed,
            &[
                ("anarchie-contribution-id", &contribution_id),
                ("anarchie-change-type", audit.change_type.as_str()),
                ("anarchie-system-id", &self.system_id),
            ],
        )?;

        Ok(CommitOutcome {
            object_id,
            version_uid,
            commit_sha: sha,
            contribution_id,
        })
    }

    /// Read the head version of a composition from the working tree.
    pub fn cat_head(&self, object_id: &str) -> Result<String> {
        let abs = self.path.join(Self::composition_rel(object_id));
        fs::read_to_string(&abs).map_err(|_| StoreError::CompositionNotFound(object_id.to_string()))
    }

    /// Read a specific historical version via git, identified by its
    /// `version_uid` (`object_id::system_id::version_tree_id`).
    pub fn cat_version(&self, version_uid: &str) -> Result<String> {
        let (object_id, version_tree_id) = parse_version_uid(version_uid)
            .ok_or_else(|| StoreError::CompositionNotFound(version_uid.to_string()))?;
        let rel = Self::composition_rel(&object_id);
        let sha = self
            .resolve_version_commit(&rel, version_tree_id)?
            .ok_or_else(|| StoreError::CompositionNotFound(version_uid.to_string()))?;
        self.git.show(&format!("{sha}:{rel}"))
    }

    /// Map a 1-based linear `version_tree_id` to the commit that produced it.
    fn resolve_version_commit(&self, rel: &str, version_tree_id: u32) -> Result<Option<String>> {
        let log = self.git.log_path(rel)?;
        // `git log` is newest-first; reverse for oldest-first ordinal access.
        let shas: Vec<&str> = log
            .lines()
            .filter_map(|line| line.split('\t').next())
            .collect();
        let idx = shas.len().checked_sub(version_tree_id as usize);
        Ok(idx.and_then(|i| shas.get(i)).map(|s| s.to_string()))
    }

    /// History of a composition as `(sha, iso_date, subject)` rows, newest
    /// first.
    pub fn log(&self, object_id: &str) -> Result<Vec<LogEntry>> {
        let rel = Self::composition_rel(object_id);
        let raw = self.git.log_path(&rel)?;
        if raw.trim().is_empty() {
            return Err(StoreError::CompositionNotFound(object_id.to_string()));
        }
        let total = raw.lines().count() as u32;
        let entries = raw
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let mut parts = line.splitn(3, '\t');
                let sha = parts.next()?.to_string();
                let date = parts.next().unwrap_or_default().to_string();
                let subject = parts.next().unwrap_or_default().to_string();
                // Newest line is index 0 -> highest version number.
                let version_tree_id = total - i as u32;
                Some(LogEntry {
                    version_uid: format!("{object_id}::{}::{version_tree_id}", self.system_id),
                    commit_sha: sha,
                    time_committed: date,
                    subject,
                })
            })
            .collect();
        Ok(entries)
    }

    /// Diff two versions of a composition (each a `version_tree_id`).
    pub fn diff(&self, object_id: &str, from: u32, to: u32) -> Result<String> {
        let rel = Self::composition_rel(object_id);
        let from_sha = self
            .resolve_version_commit(&rel, from)?
            .ok_or_else(|| StoreError::CompositionNotFound(format!("{object_id} v{from}")))?;
        let to_sha = self
            .resolve_version_commit(&rel, to)?
            .ok_or_else(|| StoreError::CompositionNotFound(format!("{object_id} v{to}")))?;
        self.git.diff(&from_sha, &to_sha, &rel)
    }

    /// List the object ids of all compositions in this EHR.
    pub fn list_compositions(&self) -> Result<Vec<String>> {
        let dir = self.path.join("compositions");
        let mut ids = Vec::new();
        if !dir.exists() {
            return Ok(ids);
        }
        let entries = fs::read_dir(&dir).map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| StoreError::Io {
                path: dir.clone(),
                source,
            })?;
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    ids.push(name.to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }
}

/// One row of composition history.
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub version_uid: String,
    pub commit_sha: String,
    pub time_committed: String,
    pub subject: String,
}

/// Split a `version_uid` into `(object_id, version_tree_id)` for the linear MVP
/// case. Returns `None` if the shape is unexpected.
fn parse_version_uid(version_uid: &str) -> Option<(String, u32)> {
    let parts: Vec<&str> = version_uid.split("::").collect();
    if parts.len() != 3 {
        return None;
    }
    let version_tree_id = parts[2].parse().ok()?;
    Some((parts[0].to_string(), version_tree_id))
}

fn to_pretty_json<T: Serialize>(value: &T) -> Result<String> {
    let mut s = serde_json::to_string_pretty(value)?;
    s.push('\n');
    Ok(s)
}

fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}
