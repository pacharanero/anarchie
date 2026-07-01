// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! The clinical-record lifecycle: `init`, `ehr`, `commit`, `cat`, `log`,
//! `diff`, and `fsck`.

use std::path::Path;

use anyhow::{bail, Context, Result};
use serde_json::json;

use super::{emit, load, open_deployment, print_report, Format};
use crate::store::{Audit, ChangeType, Deployment, DeploymentConfig, StoreError};

pub(crate) fn init(format: Format, path: &Path, system_id: &str, minimal: bool) -> Result<()> {
    let config = DeploymentConfig::new(system_id);
    let deployment = Deployment::init(path, config).context("initialising deployment")?;
    let root = deployment.root().display().to_string();
    let sys = deployment.config().system_id.clone();
    let templates: Vec<String> = if minimal {
        Vec::new()
    } else {
        deployment
            .install_starter_templates()
            .context("installing starter templates")?
    };

    let value = json!({ "root": root, "system_id": sys, "starter_templates": templates });
    emit(format, &value, || {
        println!("Initialised anarchie deployment at {root}");
        println!("  system_id: {sys}");
        if minimal {
            println!("  starter templates: none (--minimal)");
        } else {
            println!("  starter templates ({}):", templates.len());
            for id in &templates {
                println!("    - {id}");
            }
        }
    })
}

pub(crate) fn ehr(format: Format, command: super::EhrCommand) -> Result<()> {
    let deployment = open_deployment()?;
    match command {
        super::EhrCommand::New { committer, email } => {
            let audit = Audit::now(committer, email, ChangeType::Creation, "Create EHR");
            let repo = deployment.create_ehr(&audit).context("creating EHR")?;
            let id = repo.ehr_id().to_string();
            emit(format, &json!({ "ehr_id": id }), || println!("{id}"))
        }
        super::EhrCommand::List => {
            let ids = deployment.list_ehrs().context("listing EHRs")?;
            emit(format, &json!(ids), || {
                for id in &ids {
                    println!("{id}");
                }
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn commit(
    format: Format,
    ehr_id: &str,
    file: &Path,
    object_id: Option<String>,
    committer: &str,
    email: &str,
    message: Option<String>,
    no_validate: bool,
) -> Result<()> {
    let deployment = open_deployment()?;
    let repo = deployment.open_ehr(ehr_id).context("opening EHR")?;
    let composition = load(file)?;
    let change_type = if object_id.is_some() {
        ChangeType::Modification
    } else {
        ChangeType::Creation
    };
    let audit = Audit::now(
        committer,
        email,
        change_type,
        message.as_deref().unwrap_or(""),
    );
    let result = if no_validate {
        repo.commit_composition_unchecked(composition, object_id, &audit)
    } else {
        repo.commit_composition(composition, object_id, &audit)
    };
    let outcome = match result {
        Ok(outcome) => outcome,
        Err(StoreError::Invalid(report)) => {
            eprintln!("Rejected: composition failed validation");
            print_report(&report);
            bail!("{} validation error(s)", report.error_count());
        }
        Err(err) => return Err(anyhow::Error::new(err).context("committing composition")),
    };

    let value = json!({
        "version_uid": outcome.version_uid,
        "object_id": outcome.object_id,
        "commit": outcome.commit_sha,
        "contribution_id": outcome.contribution_id,
    });
    emit(format, &value, || {
        println!("Committed {}", outcome.version_uid);
        println!("  object_id:       {}", outcome.object_id);
        println!("  commit:          {}", outcome.commit_sha);
        println!("  contribution_id: {}", outcome.contribution_id);
    })
}

pub(crate) fn cat(ehr_id: &str, target: &str) -> Result<()> {
    let deployment = open_deployment()?;
    let repo = deployment.open_ehr(ehr_id).context("opening EHR")?;
    // A version_uid has the form object_id::system_id::version_tree_id.
    let body = if target.contains("::") {
        repo.cat_version(target).context("reading version")?
    } else {
        repo.cat_head(target).context("reading head version")?
    };
    // The Composition is canonical JSON already; it is the artefact, printed
    // verbatim regardless of --format.
    print!("{body}");
    if !body.ends_with('\n') {
        println!();
    }
    Ok(())
}

pub(crate) fn log(format: Format, ehr_id: &str, object_id: &str) -> Result<()> {
    let deployment = open_deployment()?;
    let repo = deployment.open_ehr(ehr_id).context("opening EHR")?;
    let entries = repo.log(object_id).context("reading history")?;
    let value = json!(entries
        .iter()
        .map(|e| json!({
            "version_uid": e.version_uid,
            "time_committed": e.time_committed,
            "subject": e.subject,
            "commit": e.commit_sha,
        }))
        .collect::<Vec<_>>());
    emit(format, &value, || {
        for entry in &entries {
            println!(
                "{}  {}  {}",
                entry.version_uid, entry.time_committed, entry.subject
            );
            println!("  commit {}", entry.commit_sha);
        }
    })
}

pub(crate) fn diff(
    format: Format,
    ehr_id: &str,
    object_id: &str,
    from: u32,
    to: u32,
) -> Result<()> {
    if from == 0 || to == 0 {
        bail!("version_tree_id is 1-based; v0 does not exist");
    }
    let deployment = open_deployment()?;
    let repo = deployment.open_ehr(ehr_id).context("opening EHR")?;
    let diff = repo.diff(object_id, from, to).context("diffing versions")?;
    let value = json!({ "from": from, "to": to, "diff": diff });
    emit(format, &value, || print!("{diff}"))
}

pub(crate) fn fsck(format: Format) -> Result<()> {
    let deployment = open_deployment()?;
    let report = deployment.fsck().context("checking store integrity")?;
    let issues: Vec<_> = report
        .issues
        .iter()
        .map(|i| {
            json!({
                "ehr_id": i.ehr_id,
                "object_id": i.object_id,
                "problems": i.problems,
            })
        })
        .collect();
    let value = json!({
        "ehrs": report.ehrs,
        "compositions": report.compositions,
        "issues": issues,
    });
    emit(format, &value, || {
        println!(
            "Checked {} composition(s) across {} EHR(s)",
            report.compositions, report.ehrs
        );
        for issue in &report.issues {
            println!("  ✗ {}/{}", issue.ehr_id, issue.object_id);
            for problem in &issue.problems {
                println!("      {problem}");
            }
        }
        if report.is_clean() {
            println!("Store is clean.");
        } else {
            println!("{} composition(s) with problems.", report.issues.len());
        }
    })?;
    if !report.is_clean() {
        std::process::exit(1);
    }
    Ok(())
}
