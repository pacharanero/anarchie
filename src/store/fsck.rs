// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! `fsck` - a full integrity check of the store against the Reference Model.
//!
//! Because the canonical files are the system of record, their integrity can be
//! verified at any time independently of the index or any cache: walk every
//! EHR's head Compositions, parse each as canonical JSON, and validate it
//! against the RM (and its claimed Operational Template, if registered).
//! Anything that fails is reported. See `specs/roadmap.md` (Phase 6).

use crate::rm::Composition;

use crate::store::deployment::Deployment;
use crate::store::error::Result;

/// One Composition that failed an integrity check.
#[derive(Clone, Debug)]
pub struct FsckIssue {
    pub ehr_id: String,
    pub object_id: String,
    /// Human-readable problem descriptions (parse failure or RM/OPT violations).
    pub problems: Vec<String>,
}

/// The outcome of an `fsck` pass.
#[derive(Clone, Debug, Default)]
pub struct FsckReport {
    /// Number of EHRs walked.
    pub ehrs: usize,
    /// Number of head Compositions checked.
    pub compositions: usize,
    /// The Compositions that failed, with their problems.
    pub issues: Vec<FsckIssue>,
}

impl FsckReport {
    /// Whether the store is clean (no issues found).
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

impl Deployment {
    /// Check every EHR's head Compositions against the RM and, where the claimed
    /// template is registered, against that Operational Template. Returns a
    /// report listing any Composition that fails to parse or validate.
    pub fn fsck(&self) -> Result<FsckReport> {
        let mut report = FsckReport::default();

        for ehr_id in self.list_ehrs()? {
            report.ehrs += 1;
            let repo = self.open_ehr(&ehr_id)?;
            for object_id in repo.list_compositions()? {
                report.compositions += 1;
                let problems = self.check_composition(&repo, &object_id)?;
                if !problems.is_empty() {
                    report.issues.push(FsckIssue {
                        ehr_id: ehr_id.clone(),
                        object_id,
                        problems,
                    });
                }
            }
        }
        Ok(report)
    }

    /// Validate one head Composition, returning a list of problems (empty when
    /// the Composition is intact and conformant).
    fn check_composition(
        &self,
        repo: &crate::store::deployment::EhrRepo,
        object_id: &str,
    ) -> Result<Vec<String>> {
        let json = repo.cat_head(object_id)?;
        let composition: Composition = match crate::rm::from_canonical_str(&json) {
            Ok(c) => c,
            Err(e) => return Ok(vec![format!("does not parse as a Composition: {e}")]),
        };

        // Validate against the claimed template when it is registered, otherwise
        // against the RM alone.
        let template = match composition.archetype_details.template_id.as_ref() {
            Some(t) => self.get_template(&t.value)?,
            None => None,
        };
        let report = crate::validate::validate(&composition, template.as_ref());

        Ok(report
            .violations
            .iter()
            .filter(|v| v.severity == crate::validate::Severity::Error)
            .map(|v| format!("{}: {} ({})", v.rm_path, v.message, v.constraint))
            .collect())
    }
}
