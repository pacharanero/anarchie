// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! The schema layer: `validate`, `template`, and `pack` (Operational
//! Templates and the archetype packs that install them).

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::json;

use super::{emit, load, open_deployment, print_report, Format};
use crate::validate::Opt;

pub(crate) fn validate(format: Format, file: &Path, template_id: Option<&str>) -> Result<()> {
    let composition = load(file)?;
    let opt = match template_id {
        Some(id) => {
            let deployment = open_deployment()?;
            let opt = deployment
                .get_template(id)
                .context("loading template")?
                .ok_or_else(|| anyhow::anyhow!("template `{id}` is not registered"))?;
            Some(opt)
        }
        None => None,
    };
    let report = crate::validate::validate(&composition, opt.as_ref());
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        Format::Text => {
            print_report(&report);
            if report.valid {
                println!("valid");
            }
        }
    }
    if report.error_count() > 0 {
        std::process::exit(1);
    }
    Ok(())
}

pub(crate) fn template(format: Format, command: super::TemplateCommand) -> Result<()> {
    let deployment = open_deployment()?;
    match command {
        super::TemplateCommand::Add { file } => {
            let json_text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let opt = Opt::from_json(&json_text)
                .with_context(|| format!("parsing {} as an anarchie OPT", file.display()))?;
            let id = deployment
                .add_template(&opt)
                .context("registering template")?;
            emit(format, &json!({ "template_id": id }), || {
                println!("Registered template {id}")
            })
        }
        super::TemplateCommand::List => {
            let ids = deployment.list_templates().context("listing templates")?;
            emit(format, &json!(ids), || {
                for id in &ids {
                    println!("{id}");
                }
            })
        }
    }
}

pub(crate) fn pack(format: Format, command: super::PackCommand) -> Result<()> {
    match command {
        super::PackCommand::Add { source } => {
            let deployment = open_deployment()?;
            let ids = deployment
                .install_pack(&source)
                .with_context(|| format!("installing pack `{source}`"))?;
            let value = json!({ "pack": source, "templates": ids });
            emit(format, &value, || {
                println!("Installed {} template(s) from pack `{source}`:", ids.len());
                for id in &ids {
                    println!("  - {id}");
                }
            })
        }
        super::PackCommand::List => {
            let packs = crate::store::bundled_packs();
            emit(format, &json!(packs), || {
                println!("Bundled packs:");
                for name in packs {
                    println!("  - {name}");
                }
            })
        }
    }
}
