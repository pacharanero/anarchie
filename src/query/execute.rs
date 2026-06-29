// SPDX-License-Identifier: AGPL-3.0-or-later
//! AQL execution: translate a parsed query into SQL over the path index, run
//! it, and shape the result into an openEHR-style AQL `ResultSet`.
//!
//! The translation rests on the index's design ([`crate::query::index`]): every leaf
//! value is keyed by its composition-rooted canonical path, and every row
//! carries the archetype id of the ENTRY it lives under. So an identified path
//! resolves to an exact `path` lookup, and a `CONTAINS OBSERVATION o[id]` maps
//! to an exact `entry_archetype` match - no JSON walking at query time.

use std::collections::BTreeMap;

use rusqlite::types::{ToSql, ToSqlOutput, Value as SqlValue};
use serde::Serialize;
use serde_json::Value;

use crate::query::aql::{
    parse, AggFunc, AqlQuery, CompareOp, Container, IdentifiedPath, SelectExpr, SelectKind,
    Terminal, WhereExpr,
};
use crate::query::error::{QueryError, Result};
use crate::query::index::Index;

/// `$`-parameter bindings supplied with a query (name → value text).
pub type Params = BTreeMap<String, String>;

/// An openEHR-style AQL result set: the query, its columns, and the rows.
#[derive(Clone, Debug, Serialize)]
pub struct ResultSet {
    pub q: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
}

/// One result column: a display name and the path it projects.
#[derive(Clone, Debug, Serialize)]
pub struct Column {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Parse, translate and execute `aql` against `index`, returning the result set.
pub fn execute(index: &Index, aql: &str, params: &Params) -> Result<ResultSet> {
    let query = parse(aql).map_err(QueryError::Parse)?;
    let vars = VarMap::from_chain(&query.from)?;
    let plan = Plan::build(&query, &vars, params)?;

    let mut stmt = index.conn().prepare(&plan.sql)?;
    let col_count = plan.columns.len();
    let rows = stmt
        .query_map(rusqlite::params_from_iter(plan.binds.iter()), |row| {
            let mut out = Vec::with_capacity(col_count);
            for i in 0..col_count {
                out.push(sql_to_json(row, i));
            }
            Ok(out)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(ResultSet {
        q: aql.to_string(),
        columns: plan.columns,
        rows,
    })
}

/// Read column `i` of a result row into a JSON value (number, string, or null).
fn sql_to_json(row: &rusqlite::Row, i: usize) -> Value {
    match row.get_ref(i) {
        Ok(rusqlite::types::ValueRef::Null) => Value::Null,
        Ok(rusqlite::types::ValueRef::Integer(n)) => Value::from(n),
        Ok(rusqlite::types::ValueRef::Real(f)) => Value::from(f),
        Ok(rusqlite::types::ValueRef::Text(t)) => {
            Value::from(String::from_utf8_lossy(t).into_owned())
        }
        Ok(rusqlite::types::ValueRef::Blob(_)) | Err(_) => Value::Null,
    }
}

/// What an identified path resolves to in the index.
enum SqlRef {
    /// A column on the `composition` table (currently only `ehr_id`).
    Column(&'static str),
    /// An exact `path_value.path` lookup.
    Path(String),
    /// An ENTRY-level containment check on `entry_archetype`.
    Containment(String),
}

#[derive(Clone, Copy)]
enum VarKind {
    Ehr,
    Composition,
    Entry,
}

struct VarBinding {
    kind: VarKind,
    archetype: Option<String>,
}

struct VarMap {
    vars: BTreeMap<String, VarBinding>,
}

impl VarMap {
    fn from_chain(chain: &[Container]) -> Result<Self> {
        let mut vars = BTreeMap::new();
        for c in chain {
            let kind = match c.rm_type.as_str() {
                "EHR" => VarKind::Ehr,
                "COMPOSITION" => VarKind::Composition,
                _ => VarKind::Entry,
            };
            if let Some(v) = &c.variable {
                vars.insert(
                    v.clone(),
                    VarBinding {
                        kind,
                        archetype: c.archetype_id.clone(),
                    },
                );
            }
        }
        Ok(Self { vars })
    }

    fn resolve(&self, ipath: &IdentifiedPath) -> Result<SqlRef> {
        let binding = self.vars.get(&ipath.variable).ok_or_else(|| {
            QueryError::Unsupported(format!("unknown variable `{}`", ipath.variable))
        })?;
        match binding.kind {
            VarKind::Ehr => match ipath.path.as_deref() {
                Some("ehr_id/value") | Some("ehr_id") => Ok(SqlRef::Column("ehr_id")),
                other => Err(QueryError::Unsupported(format!(
                    "only `{}/ehr_id/value` is supported on an EHR variable (got {other:?})",
                    ipath.variable
                ))),
            },
            VarKind::Composition => match &ipath.path {
                Some(p) => Ok(SqlRef::Path(format!("/{p}"))),
                None => Err(QueryError::Unsupported(
                    "selecting a whole COMPOSITION is not supported; pick a leaf path".into(),
                )),
            },
            VarKind::Entry => {
                let arch = binding.archetype.as_ref().ok_or_else(|| {
                    QueryError::Unsupported(format!(
                        "ENTRY variable `{}` needs an [archetype] predicate to resolve its path",
                        ipath.variable
                    ))
                })?;
                match &ipath.path {
                    Some(p) => Ok(SqlRef::Path(format!("/content[{arch}]/{p}"))),
                    None => Ok(SqlRef::Containment(arch.clone())),
                }
            }
        }
    }
}

/// A bound SQL value (text or number) for positional `?` parameters.
enum Bind {
    Text(String),
    Num(f64),
}

impl ToSql for Bind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(match self {
            Bind::Text(s) => ToSqlOutput::Owned(SqlValue::Text(s.clone())),
            Bind::Num(n) => ToSqlOutput::Owned(SqlValue::Real(*n)),
        })
    }
}

/// A translated, ready-to-run query.
struct Plan {
    sql: String,
    binds: Vec<Bind>,
    columns: Vec<Column>,
}

impl Plan {
    fn build(query: &AqlQuery, vars: &VarMap, params: &Params) -> Result<Self> {
        let aggregate = query
            .select
            .iter()
            .any(|s| !matches!(s.kind, SelectKind::Path(_)));
        if aggregate {
            Self::build_aggregate(query, vars, params)
        } else {
            Self::build_projection(query, vars, params)
        }
    }

    fn build_aggregate(query: &AqlQuery, vars: &VarMap, params: &Params) -> Result<Self> {
        let mut binds = Vec::new();
        let mut select_parts = Vec::new();
        let mut columns = Vec::new();

        for (i, sel) in query.select.iter().enumerate() {
            let (expr, col) = aggregate_select(sel, vars, &mut binds, i)?;
            select_parts.push(expr);
            columns.push(col);
        }

        let where_sql = build_where(query, vars, params, &mut binds)?;
        let sql = format!(
            "SELECT {} FROM composition cmp{}",
            select_parts.join(", "),
            where_clause(&where_sql)
        );
        Ok(Self {
            sql,
            binds,
            columns,
        })
    }

    fn build_projection(query: &AqlQuery, vars: &VarMap, params: &Params) -> Result<Self> {
        let mut joins = Vec::new();
        let mut select_parts = Vec::new();
        let mut columns = Vec::new();
        let mut binds = Vec::new();

        // SELECT columns first (their joins push binds in order).
        for (i, sel) in query.select.iter().enumerate() {
            let SelectKind::Path(ipath) = &sel.kind else {
                return Err(QueryError::Unsupported(
                    "cannot mix aggregates with plain paths in SELECT".into(),
                ));
            };
            match vars.resolve(ipath)? {
                SqlRef::Column(col) => {
                    select_parts.push(format!("cmp.{col}"));
                }
                SqlRef::Path(path) => {
                    let alias = format!("pv{i}");
                    joins.push(join_clause(&alias, &path, &mut binds));
                    // Prefer the numeric projection, fall back to text.
                    select_parts.push(format!("COALESCE({alias}.value_num, {alias}.value_text)"));
                }
                SqlRef::Containment(_) => {
                    return Err(QueryError::Unsupported(
                        "cannot SELECT a bare ENTRY; select a leaf path under it".into(),
                    ));
                }
            }
            columns.push(Column {
                name: sel.alias.clone().unwrap_or_else(|| format!("#{i}")),
                path: column_path(ipath),
            });
        }

        // ORDER BY joins (also in the FROM section, binds pushed after select joins).
        let mut order_parts = Vec::new();
        for (k, ord) in query.order_by.iter().enumerate() {
            match vars.resolve(&ord.path)? {
                SqlRef::Column(col) => {
                    order_parts.push(format!("cmp.{col} {}", dir(ord.descending)));
                }
                SqlRef::Path(path) => {
                    let alias = format!("oj{k}");
                    joins.push(join_clause(&alias, &path, &mut binds));
                    order_parts.push(format!(
                        "COALESCE({alias}.value_num, {alias}.value_text) {}",
                        dir(ord.descending)
                    ));
                }
                SqlRef::Containment(_) => {
                    return Err(QueryError::Unsupported(
                        "cannot ORDER BY a bare ENTRY".into(),
                    ));
                }
            }
        }

        let where_sql = build_where(query, vars, params, &mut binds)?;

        let mut sql = format!(
            "SELECT {}{} FROM composition cmp {}{}",
            if query.distinct { "DISTINCT " } else { "" },
            select_parts.join(", "),
            joins.join(" "),
            where_clause(&where_sql),
        );
        if !order_parts.is_empty() {
            sql.push_str(&format!(" ORDER BY {}", order_parts.join(", ")));
        }
        if let Some(limit) = query.limit.or(query.top) {
            sql.push_str(&format!(" LIMIT {limit}"));
            if let Some(offset) = query.offset {
                sql.push_str(&format!(" OFFSET {offset}"));
            }
        }
        Ok(Self {
            sql,
            binds,
            columns,
        })
    }
}

/// Build the `AGG(...)` SELECT expression and its column for one aggregate.
fn aggregate_select(
    sel: &SelectExpr,
    vars: &VarMap,
    binds: &mut Vec<Bind>,
    i: usize,
) -> Result<(String, Column)> {
    match &sel.kind {
        SelectKind::CountStar => Ok((
            "COUNT(*)".to_string(),
            Column {
                name: sel.alias.clone().unwrap_or_else(|| "#count".into()),
                path: None,
            },
        )),
        SelectKind::Aggregate { func, arg } => {
            let inner = match vars.resolve(arg)? {
                SqlRef::Column(col) => format!("cmp.{col}"),
                SqlRef::Path(path) => {
                    binds.push(Bind::Text(path));
                    let value = if matches!(func, AggFunc::Count) {
                        "1"
                    } else {
                        "value_num"
                    };
                    format!(
                        "(SELECT {value} FROM path_value w \
                          WHERE w.ehr_id=cmp.ehr_id AND w.comp_id=cmp.comp_id AND w.path=? LIMIT 1)"
                    )
                }
                SqlRef::Containment(_) => {
                    return Err(QueryError::Unsupported(
                        "cannot aggregate a bare ENTRY".into(),
                    ))
                }
            };
            Ok((
                format!("{}({inner})", func.sql()),
                Column {
                    name: sel.alias.clone().unwrap_or_else(|| format!("#{i}")),
                    path: column_path(arg),
                },
            ))
        }
        SelectKind::Path(_) => unreachable!("aggregate_select called on a path"),
    }
}

/// A `LEFT JOIN path_value <alias> ON … AND <alias>.path = ?` clause.
fn join_clause(alias: &str, path: &str, binds: &mut Vec<Bind>) -> String {
    binds.push(Bind::Text(path.to_string()));
    format!(
        "LEFT JOIN path_value {alias} ON {alias}.ehr_id=cmp.ehr_id \
         AND {alias}.comp_id=cmp.comp_id AND {alias}.path=?"
    )
}

/// Combine the FROM containment filters with the optional WHERE into one SQL
/// boolean (empty string when there is nothing to filter).
fn build_where(
    query: &AqlQuery,
    vars: &VarMap,
    params: &Params,
    binds: &mut Vec<Bind>,
) -> Result<String> {
    let mut clauses = Vec::new();

    // Every contained ENTRY with an archetype implies a containment filter.
    for c in &query.from {
        if matches!(c.rm_type.as_str(), "EHR" | "COMPOSITION") {
            continue;
        }
        if let Some(arch) = &c.archetype_id {
            binds.push(Bind::Text(arch.clone()));
            clauses.push(
                "EXISTS (SELECT 1 FROM path_value cx WHERE cx.ehr_id=cmp.ehr_id \
                 AND cx.comp_id=cmp.comp_id AND cx.entry_archetype=?)"
                    .to_string(),
            );
        }
    }

    if let Some(w) = &query.where_clause {
        clauses.push(translate_where(w, vars, params, binds)?);
    }

    Ok(clauses.join(" AND "))
}

fn translate_where(
    expr: &WhereExpr,
    vars: &VarMap,
    params: &Params,
    binds: &mut Vec<Bind>,
) -> Result<String> {
    match expr {
        WhereExpr::And(l, r) => Ok(format!(
            "({} AND {})",
            translate_where(l, vars, params, binds)?,
            translate_where(r, vars, params, binds)?
        )),
        WhereExpr::Or(l, r) => Ok(format!(
            "({} OR {})",
            translate_where(l, vars, params, binds)?,
            translate_where(r, vars, params, binds)?
        )),
        WhereExpr::Not(inner) => Ok(format!(
            "(NOT {})",
            translate_where(inner, vars, params, binds)?
        )),
        WhereExpr::Exists(path) => match vars.resolve(path)? {
            SqlRef::Column(col) => Ok(format!("cmp.{col} IS NOT NULL")),
            SqlRef::Path(p) => {
                binds.push(Bind::Text(p));
                Ok(leaf_exists("w.path=?"))
            }
            SqlRef::Containment(arch) => {
                binds.push(Bind::Text(arch));
                Ok(leaf_exists("w.entry_archetype=?"))
            }
        },
        WhereExpr::Compare { path, op, value } => compare(path, *op, value, vars, params, binds),
        WhereExpr::Matches { path, values } => matches_clause(path, values, vars, params, binds),
        WhereExpr::Like { path, pattern } => {
            let SqlRef::Path(p) = vars.resolve(path)? else {
                return Err(QueryError::Unsupported(
                    "LIKE is only supported on a leaf path".into(),
                ));
            };
            binds.push(Bind::Text(p));
            binds.push(Bind::Text(pattern.clone()));
            Ok(leaf_exists("w.path=? AND w.value_text LIKE ?"))
        }
    }
}

fn compare(
    path: &IdentifiedPath,
    op: CompareOp,
    value: &Terminal,
    vars: &VarMap,
    params: &Params,
    binds: &mut Vec<Bind>,
) -> Result<String> {
    let resolved = vars.resolve(path)?;
    let (column, bind) = terminal_to_bind(value, params)?;
    match resolved {
        SqlRef::Column(col) => {
            binds.push(bind);
            Ok(format!("cmp.{col} {} ?", op.sql()))
        }
        SqlRef::Path(p) => {
            binds.push(Bind::Text(p));
            binds.push(bind);
            Ok(leaf_exists(&format!(
                "w.path=? AND w.{column} {} ?",
                op.sql()
            )))
        }
        SqlRef::Containment(_) => Err(QueryError::Unsupported(
            "cannot compare a bare ENTRY; compare a leaf path".into(),
        )),
    }
}

fn matches_clause(
    path: &IdentifiedPath,
    values: &[Terminal],
    vars: &VarMap,
    params: &Params,
    binds: &mut Vec<Bind>,
) -> Result<String> {
    let SqlRef::Path(p) = vars.resolve(path)? else {
        return Err(QueryError::Unsupported(
            "MATCHES is only supported on a leaf path".into(),
        ));
    };
    binds.push(Bind::Text(p));
    let mut placeholders = Vec::new();
    for v in values {
        let (_, bind) = terminal_to_bind(v, params)?;
        // MATCHES compares against the text projection (codes, coded text).
        binds.push(match bind {
            Bind::Num(n) => Bind::Text(crate::query::index::fmt_num_pub(n)),
            other => other,
        });
        placeholders.push("?");
    }
    Ok(leaf_exists(&format!(
        "w.path=? AND w.value_text IN ({})",
        placeholders.join(", ")
    )))
}

/// Pick the comparison column (`value_num`/`value_text`) and bound value for a
/// terminal. Numbers compare numerically; strings/bools textually; a `$param`
/// compares numerically when its value parses as a number.
fn terminal_to_bind(value: &Terminal, params: &Params) -> Result<(&'static str, Bind)> {
    Ok(match value {
        Terminal::Number(n) => ("value_num", Bind::Num(*n)),
        Terminal::String(s) => ("value_text", Bind::Text(s.clone())),
        Terminal::Bool(b) => ("value_text", Bind::Text(b.to_string())),
        Terminal::Param(name) => {
            let raw = params
                .get(name)
                .ok_or_else(|| QueryError::MissingParameter(name.clone()))?;
            match raw.parse::<f64>() {
                Ok(n) => ("value_num", Bind::Num(n)),
                Err(_) => ("value_text", Bind::Text(raw.clone())),
            }
        }
    })
}

/// Wrap a per-composition leaf predicate in a correlated `EXISTS`.
fn leaf_exists(inner: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM path_value w \
         WHERE w.ehr_id=cmp.ehr_id AND w.comp_id=cmp.comp_id AND {inner})"
    )
}

fn where_clause(body: &str) -> String {
    if body.is_empty() {
        String::new()
    } else {
        format!(" WHERE {body}")
    }
}

fn dir(descending: bool) -> &'static str {
    if descending {
        "DESC"
    } else {
        "ASC"
    }
}

/// Reconstruct the textual identified path for a result column header.
fn column_path(ipath: &IdentifiedPath) -> Option<String> {
    Some(match &ipath.path {
        Some(p) => format!("{}/{p}", ipath.variable),
        None => ipath.variable.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const BP: &str = "openEHR-EHR-OBSERVATION.blood_pressure.v2";
    const SYS: &str = "data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude";

    fn element(node: &str, magnitude: f64) -> Value {
        json!({
            "_type": "ELEMENT",
            "archetype_node_id": node,
            "value": { "_type": "DV_QUANTITY", "magnitude": magnitude, "units": "mm[Hg]" }
        })
    }

    fn bp(name: &str, systolic: f64, diastolic: f64) -> Value {
        json!({
            "_type": "COMPOSITION",
            "name": { "_type": "DV_TEXT", "value": name },
            "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
            "uid": { "_type": "OBJECT_VERSION_ID", "value": "x::sys::1" },
            "content": [{
                "_type": "OBSERVATION",
                "archetype_node_id": BP,
                "data": { "_type": "HISTORY", "archetype_node_id": "at0001", "events": [{
                    "_type": "POINT_EVENT", "archetype_node_id": "at0006",
                    "data": { "_type": "ITEM_TREE", "archetype_node_id": "at0003", "items": [
                        element("at0004", systolic),
                        element("at0005", diastolic)
                    ]}
                }]}
            }]
        })
    }

    fn fixture() -> Index {
        let index = Index::open_in_memory().unwrap();
        index
            .index_composition("ehr1", "c1", &bp("Low", 120.0, 80.0))
            .unwrap();
        index
            .index_composition("ehr1", "c2", &bp("High", 160.0, 95.0))
            .unwrap();
        index
    }

    fn run(index: &Index, aql: &str) -> ResultSet {
        execute(index, aql, &Params::new()).expect("query runs")
    }

    #[test]
    fn where_filters_on_a_numeric_leaf() {
        let index = fixture();
        let rs = run(
            &index,
            &format!(
                "SELECT o/{SYS} AS systolic \
                 FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o[{BP}] \
                 WHERE o/{SYS} > 140"
            ),
        );
        assert_eq!(rs.rows.len(), 1, "only the 160 systolic passes");
        assert_eq!(rs.rows[0][0], json!(160.0));
        assert_eq!(rs.columns[0].name, "systolic");
    }

    #[test]
    fn count_star_counts_matching_compositions() {
        let index = fixture();
        let rs = run(
            &index,
            &format!("SELECT COUNT(*) FROM COMPOSITION c CONTAINS OBSERVATION o[{BP}]"),
        );
        assert_eq!(rs.rows[0][0], json!(2));
    }

    #[test]
    fn order_by_and_limit() {
        let index = fixture();
        let rs = run(
            &index,
            &format!(
                "SELECT o/{SYS} \
                 FROM COMPOSITION c CONTAINS OBSERVATION o[{BP}] \
                 ORDER BY o/{SYS} DESC LIMIT 1"
            ),
        );
        assert_eq!(rs.rows.len(), 1);
        assert_eq!(rs.rows[0][0], json!(160.0));
    }

    #[test]
    fn parameter_binding_resolves_at_execution() {
        let index = fixture();
        let mut params = Params::new();
        params.insert("min".into(), "150".into());
        let rs = execute(
            &index,
            &format!(
                "SELECT o/{SYS} FROM COMPOSITION c CONTAINS OBSERVATION o[{BP}] WHERE o/{SYS} >= $min"
            ),
            &params,
        )
        .unwrap();
        assert_eq!(rs.rows.len(), 1);
        assert_eq!(rs.rows[0][0], json!(160.0));
    }

    #[test]
    fn avg_aggregate_over_compositions() {
        let index = fixture();
        let rs = run(
            &index,
            &format!("SELECT AVG(o/{SYS}) AS mean FROM COMPOSITION c CONTAINS OBSERVATION o[{BP}]"),
        );
        assert_eq!(rs.rows[0][0], json!(140.0));
    }

    #[test]
    fn missing_parameter_is_an_error() {
        let index = fixture();
        let err = execute(
            &index,
            &format!(
                "SELECT o/{SYS} FROM COMPOSITION c CONTAINS OBSERVATION o[{BP}] WHERE o/{SYS} > $x"
            ),
            &Params::new(),
        )
        .unwrap_err();
        assert!(matches!(err, QueryError::MissingParameter(p) if p == "x"));
    }
}
