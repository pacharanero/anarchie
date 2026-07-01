// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! The query layer: `index`, `aql` (ad-hoc), and `query` (stored, named).

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{emit, index_db_path, open_deployment, parse_params, Format};
use crate::store::Deployment;

pub(crate) fn index(format: Format, rebuild: bool) -> Result<()> {
    let deployment = open_deployment()?;
    let db = index_db_path(&deployment);
    let mut index = crate::query::Index::open(&db).context("opening index")?;
    let count = index
        .build(&deployment, rebuild)
        .context("building index")?;
    let db_str = db.display().to_string();
    emit(format, &json!({ "indexed": count, "db": db_str }), || {
        println!("Indexed {count} composition(s) into {db_str}")
    })
}

pub(crate) fn aql(format: Format, query: &str, params: &[String]) -> Result<()> {
    let deployment = open_deployment()?;
    let params = parse_params(params)?;
    run_aql(format, &deployment, query, &params)
}

pub(crate) fn query(format: Format, command: super::QueryCommand) -> Result<()> {
    let deployment = open_deployment()?;
    match command {
        super::QueryCommand::Add {
            name,
            file,
            version,
        } => {
            let aql = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let (name, version) =
                crate::query::stored::add(deployment.root(), &name, version.as_deref(), &aql)
                    .context("registering stored query")?;
            emit(format, &json!({ "name": name, "version": version }), || {
                println!("Registered query {name}/{version}")
            })
        }
        super::QueryCommand::List => {
            let list =
                crate::query::stored::list(deployment.root()).context("listing stored queries")?;
            let value = json!(list
                .iter()
                .map(|q| json!({ "name": q.name, "version": q.version }))
                .collect::<Vec<_>>());
            emit(format, &value, || {
                for q in &list {
                    println!("{}/{}", q.name, q.version);
                }
            })
        }
        super::QueryCommand::Run {
            name,
            version,
            params,
        } => {
            let aql = crate::query::stored::get(deployment.root(), &name, version.as_deref())
                .context("loading stored query")?;
            let params = parse_params(&params)?;
            run_aql(format, &deployment, &aql, &params)
        }
    }
}

fn run_aql(
    format: Format,
    deployment: &Deployment,
    aql: &str,
    params: &crate::query::Params,
) -> Result<()> {
    let index = crate::query::Index::open(index_db_path(deployment)).context("opening index")?;
    let result = crate::query::execute(&index, aql, params).context("executing AQL")?;
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        Format::Text => print_result_table(&serde_json::to_value(&result)?),
    }
    Ok(())
}

/// Render an openEHR ResultSet (`{q, columns, rows}`) as a tab-separated table.
fn print_result_table(result: &Value) {
    let headers: Vec<String> = result
        .get("columns")
        .and_then(Value::as_array)
        .map(|cols| {
            cols.iter()
                .map(|c| {
                    c.get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default();
    println!("{}", headers.join("\t"));
    if let Some(rows) = result.get("rows").and_then(Value::as_array) {
        for row in rows {
            if let Some(cells) = row.as_array() {
                let rendered: Vec<String> = cells.iter().map(render_cell).collect();
                println!("{}", rendered.join("\t"));
            }
        }
    }
}

fn render_cell(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
