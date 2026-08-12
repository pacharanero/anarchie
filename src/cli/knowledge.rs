// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! The Knowledge Artefacts Manager command family.

use anyhow::{bail, Context, Result};
use serde::Serialize;

use super::{Format, KnowledgeCommand};
use crate::knowledge::{
    Inventory, KnowledgeLock, KnowledgeManifest, KnowledgeStatus, KnowledgeStatusState, Resolution,
    ResolutionDecision,
};

pub(crate) fn run(format: Format, command: KnowledgeCommand) -> Result<()> {
    match command {
        KnowledgeCommand::Inventory { checkout } => {
            let inventory = Inventory::scan(&checkout)
                .with_context(|| format!("inventorying CKM checkout {}", checkout.display()))?;
            match format {
                Format::Json => println!("{}", serde_json::to_string_pretty(&inventory)?),
                Format::Text => print_summary(&inventory),
            }
            Ok(())
        }
        KnowledgeCommand::Resolve {
            checkout,
            manifest,
            lock,
        } => {
            let (manifest, lock) = deployment_paths(manifest, lock)?;
            let resolution = resolve(&checkout, &manifest)?;
            match format {
                Format::Json => println!("{}", serde_json::to_string_pretty(&resolution)?),
                Format::Text => print_resolution(&resolution, &lock),
            }
            if !resolution.is_success() {
                bail!(
                    "knowledge resolution failed with {} blocking issue(s); lock not written",
                    resolution.blockers().count()
                );
            }
            resolution
                .lock
                .write_atomic(&lock)
                .with_context(|| format!("writing knowledge lock {}", lock.display()))?;
            Ok(())
        }
        KnowledgeCommand::Why {
            artefact_id,
            checkout,
            manifest,
        } => {
            let manifest = manifest_path(manifest)?;
            let resolution = resolve(&checkout, &manifest)?;
            let explanation = WhyExplanation::new(&artefact_id, &resolution);
            match format {
                Format::Json => println!("{}", serde_json::to_string_pretty(&explanation)?),
                Format::Text => print_why(&explanation),
            }
            if explanation.decisions.is_empty() && explanation.issues.is_empty() {
                bail!(
                    "artefact `{artefact_id}` was not found in the inventory or resolution issues"
                );
            }
            Ok(())
        }
        KnowledgeCommand::Status {
            checkout,
            manifest,
            lock,
        } => {
            let (manifest, lock) = deployment_paths(manifest, lock)?;
            let inventory = Inventory::scan(&checkout)
                .with_context(|| format!("inventorying CKM checkout {}", checkout.display()))?;
            let manifest = KnowledgeManifest::from_path(&manifest)
                .with_context(|| format!("loading knowledge manifest {}", manifest.display()))?;
            let lock = if lock.exists() {
                Some(
                    KnowledgeLock::from_path(&lock)
                        .with_context(|| format!("loading knowledge lock {}", lock.display()))?,
                )
            } else {
                None
            };
            let status = KnowledgeStatus::inspect(&manifest, lock.as_ref(), &inventory)
                .context("comparing knowledge state")?;
            match format {
                Format::Json => println!("{}", serde_json::to_string_pretty(&status)?),
                Format::Text => print_status(&status),
            }
            Ok(())
        }
    }
}

fn deployment_paths(
    manifest: Option<std::path::PathBuf>,
    lock: Option<std::path::PathBuf>,
) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
    match (manifest, lock) {
        (Some(manifest), Some(lock)) => Ok((manifest, lock)),
        (manifest, lock) => {
            let deployment = super::open_deployment()?;
            Ok((
                manifest.unwrap_or_else(|| deployment.root().join("knowledge.toml")),
                lock.unwrap_or_else(|| deployment.root().join("knowledge.lock")),
            ))
        }
    }
}

fn manifest_path(manifest: Option<std::path::PathBuf>) -> Result<std::path::PathBuf> {
    match manifest {
        Some(manifest) => Ok(manifest),
        None => Ok(super::open_deployment()?.root().join("knowledge.toml")),
    }
}

fn resolve(checkout: &std::path::Path, manifest: &std::path::Path) -> Result<Resolution> {
    let inventory = Inventory::scan(checkout)
        .with_context(|| format!("inventorying CKM checkout {}", checkout.display()))?;
    let manifest = KnowledgeManifest::from_path(manifest)
        .with_context(|| format!("loading knowledge manifest {}", manifest.display()))?;
    manifest
        .resolve(&inventory)
        .context("resolving knowledge policy")
}

fn print_summary(inventory: &Inventory) {
    println!("CKM knowledge inventory");
    if let Some(revision) = &inventory.source.git_revision {
        println!("Source revision: {revision}");
    }
    if inventory.source.git_dirty == Some(true) {
        println!("Source checkout: dirty");
    }
    println!("Artifacts: {}", inventory.summary.artifacts);
    println!(
        "Archetypes: {} ({} published)",
        inventory.summary.archetypes, inventory.summary.published_archetypes
    );
    println!("Templates: {}", inventory.summary.templates);
    println!("Termsets: {}", inventory.summary.termsets);
    println!("Lifecycle states:");
    for (state, count) in &inventory.summary.lifecycle_states {
        println!("  {state}: {count}");
    }
    println!("Licences:");
    for (license, count) in &inventory.summary.licenses {
        println!("  {license}: {count}");
    }
    println!(
        "Parse issues: {}",
        inventory.summary.parse_issues.values().sum::<usize>()
    );
    for (issue, count) in &inventory.summary.parse_issues {
        println!("  {issue}: {count}");
    }
    println!("Dependency issues: {}", inventory.dependency_issues.len());
    for (kind, count) in &inventory.summary.dependency_issues {
        println!("  {kind}: {count}");
    }
}

fn print_resolution(resolution: &Resolution, lock: &std::path::Path) {
    println!("Knowledge resolution");
    println!("Knowledge base: {}", resolution.lock.knowledge.name);
    println!("Selected artefacts: {}", resolution.lock.artefacts.len());
    println!(
        "Excluded artefacts: {}",
        resolution
            .decisions
            .iter()
            .filter(|decision| !decision.included)
            .count()
    );
    println!(
        "Allowed issues: {}",
        resolution
            .issues
            .iter()
            .filter(|issue| issue.allowed)
            .count()
    );
    println!("Blocking issues: {}", resolution.blockers().count());
    for issue in resolution.blockers() {
        println!(
            "  {:?}: {} -> {}",
            issue.kind, issue.artefact_path, issue.dependency
        );
    }
    if resolution.is_success() {
        println!("Lock: {}", lock.display());
    }
}

fn print_status(status: &KnowledgeStatus) {
    println!("Knowledge status");
    println!("Knowledge base: {}", status.knowledge_name);
    println!(
        "State: {}",
        match status.state {
            KnowledgeStatusState::Current => "current",
            KnowledgeStatusState::Stale => "stale",
            KnowledgeStatusState::Unresolved => "unresolved",
        }
    );
    if let Some(selected) = status.selected_artefacts {
        println!("Selected artefacts: {selected}");
    }
    if let Some(revision) = &status.source_revision {
        println!("Source revision: {revision}");
    }
}

#[derive(Serialize)]
struct WhyExplanation<'a> {
    artefact_id: &'a str,
    decisions: Vec<&'a ResolutionDecision>,
    issues: Vec<&'a crate::knowledge::ResolutionIssue>,
}

impl<'a> WhyExplanation<'a> {
    fn new(artefact_id: &'a str, resolution: &'a Resolution) -> Self {
        let decisions = resolution.explain(artefact_id);
        let issues = resolution
            .issues
            .iter()
            .filter(|issue| {
                issue.artefact_id.as_deref() == Some(artefact_id)
                    || issue.dependency == artefact_id
                    || issue
                        .candidates
                        .iter()
                        .any(|candidate| candidate == artefact_id)
            })
            .collect();
        Self {
            artefact_id,
            decisions,
            issues,
        }
    }
}

fn print_why(explanation: &WhyExplanation<'_>) {
    println!("Knowledge explanation: {}", explanation.artefact_id);
    for decision in &explanation.decisions {
        println!(
            "  {}: {} ({:?})",
            if decision.included {
                "included"
            } else {
                "excluded"
            },
            decision.path,
            decision.reason
        );
    }
    for issue in &explanation.issues {
        println!(
            "  {} issue: {:?} (requested by {})",
            if issue.allowed { "allowed" } else { "blocking" },
            issue.kind,
            issue.artefact_path
        );
    }
}
