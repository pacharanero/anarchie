// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! The Knowledge Artefacts Manager command family.

use anyhow::{Context, Result};

use super::{Format, KnowledgeCommand};
use crate::knowledge::Inventory;

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
    }
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
