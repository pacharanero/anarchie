// SPDX-License-Identifier: AGPL-3.0-or-later
//! The path-extraction index: the read model.
//!
//! openEHR's canonical files are the system of record (the write model). To
//! answer population queries (AQL) without scanning every file, each
//! Composition is *flattened* into `(path, value)` rows in a SQLite database -
//! the read model. This is CQRS: the index is **derived and disposable**, never
//! authoritative, and can always be rebuilt from the file tree. See
//! `specs/query-engine.md`.
//!
//! Each leaf data value becomes one or more rows keyed by its
//! composition-rooted canonical archetype path, e.g.
//! `/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude`.
//! AQL identified paths resolve onto exactly these paths, so a query becomes a
//! cheap indexed lookup instead of a tree walk over JSON.

use std::path::Path;

use crate::store::{Deployment, Git};
use rusqlite::Connection;
use serde_json::Value;

use crate::query::error::Result;

/// The SQLite-backed path index.
pub struct Index {
    conn: Connection,
}

/// One flattened path-value row.
struct PathRow {
    /// The archetype id of the top-level ENTRY this row lives under (e.g.
    /// `openEHR-EHR-OBSERVATION.blood_pressure.v2`), or `None` for
    /// composition-level rows (name, context, ...). Lets a `CONTAINS
    /// OBSERVATION o[id]` filter be an exact column match.
    entry_archetype: Option<String>,
    /// The composition-rooted canonical archetype path of the value.
    path: String,
    /// The value as text (always populated for projected leaves).
    value_text: String,
    /// The value as a number, where the leaf is numeric.
    value_num: Option<f64>,
    /// The RM type of the carrying data value (`DV_QUANTITY`, ...).
    value_type: String,
    /// For coded values, the terminology id of the code.
    terminology: Option<String>,
}

/// Metadata about one indexed Composition.
struct CompMeta {
    template_id: Option<String>,
    root_archetype: String,
    name: Option<String>,
    version: i64,
}

impl Index {
    /// Open (creating if absent) the index database at `path`, ensuring schema.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let index = Self { conn };
        index.ensure_schema()?;
        Ok(index)
    }

    /// Open an in-memory index (used in tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let index = Self { conn };
        index.ensure_schema()?;
        Ok(index)
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS composition (
                 ehr_id          TEXT NOT NULL,
                 comp_id         TEXT NOT NULL,
                 version         INTEGER NOT NULL,
                 template_id     TEXT,
                 root_archetype  TEXT,
                 name            TEXT,
                 PRIMARY KEY (ehr_id, comp_id)
             );
             CREATE TABLE IF NOT EXISTS path_value (
                 ehr_id          TEXT NOT NULL,
                 comp_id         TEXT NOT NULL,
                 entry_archetype TEXT,
                 path            TEXT NOT NULL,
                 value_text      TEXT,
                 value_num       REAL,
                 value_type      TEXT,
                 terminology     TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_pv_path ON path_value(path);
             CREATE INDEX IF NOT EXISTS idx_pv_comp ON path_value(ehr_id, comp_id);
             CREATE INDEX IF NOT EXISTS idx_pv_entry ON path_value(entry_archetype);
             CREATE TABLE IF NOT EXISTS ehr_freshness (
                 ehr_id          TEXT PRIMARY KEY,
                 indexed_commit  TEXT NOT NULL
             );",
        )?;
        Ok(())
    }

    /// Re-index a whole deployment. When `rebuild` is false, EHRs whose git HEAD
    /// is unchanged since their last indexing are skipped (the freshness check);
    /// when true, every EHR is dropped and rebuilt from scratch. Returns the
    /// number of compositions (re-)indexed.
    pub fn build(&mut self, deployment: &Deployment, rebuild: bool) -> Result<usize> {
        if rebuild {
            self.conn.execute_batch(
                "DELETE FROM path_value; DELETE FROM composition; DELETE FROM ehr_freshness;",
            )?;
        }

        let mut indexed = 0usize;
        for ehr_id in deployment.list_ehrs()? {
            let repo = deployment.open_ehr(&ehr_id)?;
            let head = Git::open(repo.path()).head_sha().unwrap_or_default();

            if !rebuild && self.ehr_is_fresh(&ehr_id, &head)? {
                continue;
            }

            self.clear_ehr(&ehr_id)?;
            let tx = self.conn.transaction()?;
            for comp_id in repo.list_compositions()? {
                let json = repo.cat_head(&comp_id)?;
                let comp: Value = serde_json::from_str(&json)?;
                index_one(&tx, &ehr_id, &comp_id, &comp)?;
                indexed += 1;
            }
            tx.execute(
                "INSERT OR REPLACE INTO ehr_freshness (ehr_id, indexed_commit) VALUES (?1, ?2)",
                (&ehr_id, &head),
            )?;
            tx.commit()?;
        }
        Ok(indexed)
    }

    fn ehr_is_fresh(&self, ehr_id: &str, head: &str) -> Result<bool> {
        let stored: Option<String> = self
            .conn
            .query_row(
                "SELECT indexed_commit FROM ehr_freshness WHERE ehr_id = ?1",
                [ehr_id],
                |row| row.get(0),
            )
            .ok();
        Ok(stored.as_deref() == Some(head) && !head.is_empty())
    }

    fn clear_ehr(&self, ehr_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM path_value WHERE ehr_id = ?1", [ehr_id])?;
        self.conn
            .execute("DELETE FROM composition WHERE ehr_id = ?1", [ehr_id])?;
        Ok(())
    }

    /// Number of compositions currently in the index.
    pub fn composition_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM composition", [], |row| row.get(0))?)
    }

    /// Number of path-value rows currently in the index.
    pub fn row_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM path_value", [], |row| row.get(0))?)
    }

    /// Index a single Composition into an open connection. Exposed for tests.
    #[cfg(test)]
    pub(crate) fn index_composition(
        &self,
        ehr_id: &str,
        comp_id: &str,
        comp: &Value,
    ) -> Result<()> {
        index_one(&self.conn, ehr_id, comp_id, comp)
    }
}

/// Insert one Composition's metadata and flattened rows using `conn` (which may
/// be a transaction).
fn index_one(conn: &Connection, ehr_id: &str, comp_id: &str, comp: &Value) -> Result<()> {
    let (meta, rows) = flatten_composition(comp);
    conn.execute(
        "INSERT OR REPLACE INTO composition
             (ehr_id, comp_id, version, template_id, root_archetype, name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            ehr_id,
            comp_id,
            meta.version,
            &meta.template_id,
            &meta.root_archetype,
            &meta.name,
        ),
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO path_value
             (ehr_id, comp_id, entry_archetype, path, value_text, value_num, value_type, terminology)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    for row in rows {
        stmt.execute((
            ehr_id,
            comp_id,
            &row.entry_archetype,
            &row.path,
            &row.value_text,
            row.value_num,
            &row.value_type,
            &row.terminology,
        ))?;
    }
    Ok(())
}

/// Flatten a Composition's canonical JSON into metadata + path-value rows.
fn flatten_composition(comp: &Value) -> (CompMeta, Vec<PathRow>) {
    let meta = CompMeta {
        template_id: comp
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str)
            .map(str::to_string),
        root_archetype: comp
            .get("archetype_node_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        name: comp
            .pointer("/name/value")
            .and_then(Value::as_str)
            .map(str::to_string),
        version: comp
            .pointer("/uid/value")
            .and_then(Value::as_str)
            .and_then(version_of_uid)
            .unwrap_or(1),
    };
    let mut rows = Vec::new();
    walk(comp, "", None, &mut rows);
    (meta, rows)
}

/// The trailing `version_tree_id` of an `object::system::N` version uid.
fn version_of_uid(uid: &str) -> Option<i64> {
    uid.rsplit("::").next().and_then(|n| n.parse().ok())
}

/// Structural keys that are not part of a queryable data path.
const SKIP_KEYS: &[&str] = &[
    "_type",
    "archetype_node_id",
    "archetype_details",
    "uid",
    "links",
    "feeder_audit",
    "archetype_id",
];

/// Recursively walk a node, emitting a row for every projected leaf value.
/// `path` is the composition-rooted canonical path to `node`;
/// `entry_archetype` is the id of the top-level ENTRY currently inside.
fn walk(node: &Value, path: &str, entry_archetype: Option<&str>, rows: &mut Vec<PathRow>) {
    let Some(obj) = node.as_object() else { return };
    let ty = obj.get("_type").and_then(Value::as_str).unwrap_or("");

    if is_leaf_type(ty) {
        project_leaf(obj, ty, path, entry_archetype, rows);
        return;
    }

    for (key, child) in obj {
        if SKIP_KEYS.contains(&key.as_str()) {
            continue;
        }
        match child {
            Value::Object(_) => {
                let seg = segment(key, child);
                let child_entry = next_entry(entry_archetype, key, child);
                walk(
                    child,
                    &format!("{path}/{seg}"),
                    child_entry.as_deref(),
                    rows,
                );
            }
            Value::Array(items) => {
                for item in items {
                    if !item.is_object() {
                        continue;
                    }
                    let seg = segment(key, item);
                    let child_entry = next_entry(entry_archetype, key, item);
                    walk(item, &format!("{path}/{seg}"), child_entry.as_deref(), rows);
                }
            }
            _ => {}
        }
    }
}

/// The path segment for attribute `key` pointing at `child`: `key[node_id]` when
/// the child carries an archetype node id, otherwise bare `key`.
fn segment(key: &str, child: &Value) -> String {
    match child.get("archetype_node_id").and_then(Value::as_str) {
        Some(node_id) => format!("{key}[{node_id}]"),
        None => key.to_string(),
    }
}

/// Determine the entry archetype in scope after descending into `child`. The
/// first archetyped object under `content` becomes the entry for its subtree.
fn next_entry(current: Option<&str>, key: &str, child: &Value) -> Option<String> {
    if current.is_some() {
        return current.map(str::to_string);
    }
    if key == "content" {
        if let Some(id) = child.get("archetype_node_id").and_then(Value::as_str) {
            if id.starts_with("openEHR-") {
                return Some(id.to_string());
            }
        }
    }
    None
}

/// Whether `ty` is a data-value type whose contents we project as leaf rows
/// (rather than recursing into structurally).
fn is_leaf_type(ty: &str) -> bool {
    matches!(
        ty,
        "DV_QUANTITY"
            | "DV_COUNT"
            | "DV_PROPORTION"
            | "DV_TEXT"
            | "DV_CODED_TEXT"
            | "DV_ORDINAL"
            | "DV_DATE_TIME"
            | "DV_DATE"
            | "DV_TIME"
            | "DV_DURATION"
            | "DV_BOOLEAN"
            | "DV_URI"
            | "DV_EHR_URI"
            | "DV_IDENTIFIER"
            | "CODE_PHRASE"
    )
}

/// Project a leaf data value into one or more rows rooted at `path`.
fn project_leaf(
    obj: &serde_json::Map<String, Value>,
    ty: &str,
    path: &str,
    entry_archetype: Option<&str>,
    rows: &mut Vec<PathRow>,
) {
    let mut emit = |sub: &str, text: String, num: Option<f64>, terminology: Option<String>| {
        rows.push(PathRow {
            entry_archetype: entry_archetype.map(str::to_string),
            path: format!("{path}{sub}"),
            value_text: text,
            value_num: num,
            value_type: ty.to_string(),
            terminology,
        });
    };

    let s = |k: &str| obj.get(k).and_then(Value::as_str).map(str::to_string);
    let n = |k: &str| obj.get(k).and_then(Value::as_f64);

    match ty {
        "DV_QUANTITY" => {
            if let Some(mag) = n("magnitude") {
                emit("/magnitude", fmt_num(mag), Some(mag), None);
            }
            if let Some(units) = s("units") {
                emit("/units", units, None, None);
            }
        }
        "DV_COUNT" => {
            if let Some(mag) = n("magnitude") {
                emit("/magnitude", fmt_num(mag), Some(mag), None);
            }
        }
        "DV_PROPORTION" => {
            if let Some(num) = n("numerator") {
                emit("/numerator", fmt_num(num), Some(num), None);
            }
            if let Some(den) = n("denominator") {
                emit("/denominator", fmt_num(den), Some(den), None);
            }
        }
        "DV_TEXT" => {
            if let Some(v) = s("value") {
                emit("/value", v, None, None);
            }
        }
        "DV_CODED_TEXT" => {
            if let Some(v) = s("value") {
                emit("/value", v, None, None);
            }
            project_code_phrase(
                obj.get("defining_code"),
                path,
                "/defining_code",
                entry_archetype,
                ty,
                rows,
            );
        }
        "DV_ORDINAL" => {
            if let Some(v) = n("value") {
                emit("/value", fmt_num(v), Some(v), None);
            }
            if let Some(symbol) = obj.get("symbol") {
                project_code_phrase(
                    symbol.get("defining_code"),
                    path,
                    "/symbol/defining_code",
                    entry_archetype,
                    ty,
                    rows,
                );
            }
        }
        "DV_BOOLEAN" => {
            if let Some(b) = obj.get("value").and_then(Value::as_bool) {
                emit("/value", b.to_string(), None, None);
            }
        }
        "DV_IDENTIFIER" => {
            if let Some(id) = s("id") {
                emit("/id", id, None, None);
            }
        }
        "CODE_PHRASE" => {
            // A standalone CODE_PHRASE (e.g. language, territory).
            let terminology = obj
                .get("terminology_id")
                .and_then(|t| t.get("value"))
                .and_then(Value::as_str)
                .map(str::to_string);
            if let Some(code) = s("code_string") {
                emit("/code_string", code, None, terminology);
            }
        }
        // DV_DATE_TIME / DV_DATE / DV_TIME / DV_DURATION / DV_URI / DV_EHR_URI
        _ => {
            if let Some(v) = s("value") {
                emit("/value", v, None, None);
            }
        }
    }
}

/// Project a `CODE_PHRASE` reached at `base+suffix` (e.g. a `defining_code`).
fn project_code_phrase(
    code: Option<&Value>,
    base: &str,
    suffix: &str,
    entry_archetype: Option<&str>,
    value_type: &str,
    rows: &mut Vec<PathRow>,
) {
    let Some(code) = code.and_then(Value::as_object) else {
        return;
    };
    let terminology = code
        .get("terminology_id")
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(code_string) = code.get("code_string").and_then(Value::as_str) {
        rows.push(PathRow {
            entry_archetype: entry_archetype.map(str::to_string),
            path: format!("{base}{suffix}/code_string"),
            value_text: code_string.to_string(),
            value_num: None,
            value_type: value_type.to_string(),
            terminology,
        });
    }
}

/// Crate-internal access to [`fmt_num`] for the query translator (so a numeric
/// `MATCHES` literal compares against the same text projection the index uses).
pub(crate) fn fmt_num_pub(n: f64) -> String {
    fmt_num(n)
}

/// Format a number without a trailing `.0` for integral values, so a magnitude
/// of `142.0` indexes (and renders) as `142`.
fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bp_composition() -> Value {
        json!({
            "_type": "COMPOSITION",
            "name": { "_type": "DV_TEXT", "value": "Blood pressure" },
            "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
            "uid": { "_type": "OBJECT_VERSION_ID", "value": "abc::sys::1" },
            "archetype_details": {
                "_type": "ARCHETYPED",
                "template_id": { "_type": "TEMPLATE_ID", "value": "vital_signs_encounter.v1" }
            },
            "content": [{
                "_type": "OBSERVATION",
                "name": { "_type": "DV_TEXT", "value": "Blood pressure" },
                "archetype_node_id": "openEHR-EHR-OBSERVATION.blood_pressure.v2",
                "data": {
                    "_type": "HISTORY",
                    "archetype_node_id": "at0001",
                    "events": [{
                        "_type": "POINT_EVENT",
                        "archetype_node_id": "at0006",
                        "data": {
                            "_type": "ITEM_TREE",
                            "archetype_node_id": "at0003",
                            "items": [{
                                "_type": "ELEMENT",
                                "archetype_node_id": "at0004",
                                "value": { "_type": "DV_QUANTITY", "magnitude": 142.0, "units": "mm[Hg]" }
                            }]
                        }
                    }]
                }
            }]
        })
    }

    #[test]
    fn flatten_extracts_the_systolic_path() {
        let (meta, rows) = flatten_composition(&bp_composition());
        assert_eq!(
            meta.template_id.as_deref(),
            Some("vital_signs_encounter.v1")
        );
        assert_eq!(meta.version, 1);

        let sys = "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude";
        let row = rows
            .iter()
            .find(|r| r.path == sys)
            .expect("systolic magnitude path present");
        assert_eq!(row.value_num, Some(142.0));
        assert_eq!(row.value_text, "142");
        assert_eq!(row.value_type, "DV_QUANTITY");
        assert_eq!(
            row.entry_archetype.as_deref(),
            Some("openEHR-EHR-OBSERVATION.blood_pressure.v2")
        );
    }

    #[test]
    fn index_round_trips_through_sqlite() {
        let index = Index::open_in_memory().unwrap();
        index
            .index_composition("ehr1", "comp1", &bp_composition())
            .unwrap();
        assert_eq!(index.composition_count().unwrap(), 1);
        assert!(index.row_count().unwrap() >= 1);

        let units: String = index
            .conn()
            .query_row(
                "SELECT value_text FROM path_value WHERE path LIKE '%/value/units'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(units, "mm[Hg]");
    }
}
