// SPDX-License-Identifier: AGPL-3.0-or-later
//! Store/query operations shared by the REST and MCP front ends.
//!
//! Both servers are thin translations from a transport (HTTP, JSON-RPC) onto
//! these functions; the functions own no data, they just drive the store and
//! query engine and report a transport-neutral [`ApiError`]. See
//! `specs/rest-api.md`.

use std::fs;

use crate::query::{execute, stored, Index, Params};
use crate::rm::Composition;
use crate::store::{Audit, ChangeType, CommitOutcome, Deployment, StoreError};
use crate::validate::ValidationReport;
use serde_json::Value;

/// A transport-neutral error, mapped to HTTP status codes / JSON-RPC errors by
/// the front ends.
#[derive(Debug)]
pub enum ApiError {
    /// 400: the request body or arguments were malformed.
    BadRequest(String),
    /// 404: the addressed resource does not exist.
    NotFound(String),
    /// 412: an `If-Match` precondition failed (lost-update protection).
    PreconditionFailed(String),
    /// 422: the Composition failed RM/OPT validation.
    Validation(ValidationReport),
    /// 500: an unexpected store/engine failure.
    Internal(String),
}

impl From<StoreError> for ApiError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::EhrNotFound(_) | StoreError::CompositionNotFound(_) => {
                ApiError::NotFound(err.to_string())
            }
            StoreError::Invalid(report) => ApiError::Validation(report),
            StoreError::Rm(_) => ApiError::BadRequest(err.to_string()),
            other => ApiError::Internal(other.to_string()),
        }
    }
}

impl From<crate::query::QueryError> for ApiError {
    fn from(err: crate::query::QueryError) -> Self {
        use crate::query::QueryError::*;
        match err {
            Parse(m) => ApiError::BadRequest(format!("AQL parse error: {m}")),
            Unsupported(m) => ApiError::BadRequest(format!("unsupported AQL: {m}")),
            MissingParameter(p) => ApiError::BadRequest(format!("missing query parameter: ${p}")),
            StoredQueryNotFound(n) => ApiError::NotFound(format!("stored query `{n}` not found")),
            other => ApiError::Internal(other.to_string()),
        }
    }
}

/// Committer identity for the contribution audit (from request headers, or
/// sensible defaults). openEHR carries this in `openEHR-AUDIT_DETAILS`; the MVP
/// accepts the committer name/email/description and defaults the rest.
pub struct Committer {
    pub name: String,
    pub email: String,
    pub description: String,
}

impl Default for Committer {
    fn default() -> Self {
        Self {
            name: "anarchie".into(),
            email: "anarchie@localhost".into(),
            // Empty: the store then derives a "Create/Update composition <id>"
            // commit subject, which reads better in `log` than a generic label.
            description: String::new(),
        }
    }
}

/// The successful outcome of a commit, plus the stored Composition for clients
/// that asked for the representation.
pub struct Committed {
    pub outcome: CommitOutcome,
    pub composition: Value,
}

/// `POST /v1/ehr` - create a new EHR, returning its `ehr.json`.
pub fn create_ehr(dep: &Deployment) -> Result<Value, ApiError> {
    let audit = Audit::now(
        "anarchie",
        "anarchie@localhost",
        ChangeType::Creation,
        "Create EHR",
    );
    let repo = dep.create_ehr(&audit)?;
    read_json(&repo.path().join("ehr.json"))
}

/// `GET /v1/ehr/{ehr_id}` - fetch an EHR's `ehr.json`.
pub fn get_ehr(dep: &Deployment, ehr_id: &str) -> Result<Value, ApiError> {
    let repo = dep.open_ehr(ehr_id)?;
    read_json(&repo.path().join("ehr.json"))
}

/// `POST/PUT …/composition` - commit a Composition as a new object or, when
/// `object_id` is given, a new version of an existing one. `if_match` enforces
/// optimistic concurrency against the current head version on updates.
pub fn commit_composition(
    dep: &Deployment,
    ehr_id: &str,
    body: &str,
    object_id: Option<String>,
    if_match: Option<&str>,
    committer: &Committer,
) -> Result<Committed, ApiError> {
    let repo = dep.open_ehr(ehr_id)?;

    if let (Some(object_id), Some(if_match)) = (&object_id, if_match) {
        // ETags are sent quoted (and may be weak); compare bare version uids.
        let want = if_match.trim_start_matches("W/").trim_matches('"');
        let current = current_version_uid(&repo, object_id)?;
        if current.as_deref() != Some(want) {
            return Err(ApiError::PreconditionFailed(format!(
                "If-Match {want} does not match current version {current:?}"
            )));
        }
    }

    let composition: Composition = crate::rm::from_canonical_str(body)
        .map_err(|e| ApiError::BadRequest(format!("invalid Composition JSON: {e}")))?;

    let change_type = if object_id.is_some() {
        ChangeType::Modification
    } else {
        ChangeType::Creation
    };
    let audit = Audit::now(
        &committer.name,
        &committer.email,
        change_type,
        &committer.description,
    );

    let outcome = repo.commit_composition(composition, object_id, &audit)?;
    let composition = read_json_str(&repo.cat_head(&outcome.object_id)?)?;
    Ok(Committed {
        outcome,
        composition,
    })
}

/// `GET …/composition/{uid}` - a head object id or a full `obj::sys::N`
/// version uid.
pub fn get_composition(dep: &Deployment, ehr_id: &str, uid: &str) -> Result<Value, ApiError> {
    let repo = dep.open_ehr(ehr_id)?;
    let json = if uid.contains("::") {
        repo.cat_version(uid)?
    } else {
        repo.cat_head(uid)?
    };
    read_json_str(&json)
}

/// `GET/POST /v1/query/aql` - run an ad-hoc AQL query.
///
/// The index is refreshed incrementally first (only EHRs whose git HEAD moved
/// are re-indexed), so a Composition committed over REST is queryable straight
/// away without a separate `anarchie index` step.
pub fn run_aql(dep: &Deployment, aql: &str, params: &Params) -> Result<Value, ApiError> {
    let mut index = open_index(dep)?;
    index.build(dep, false)?;
    let result = execute(&index, aql, params)?;
    Ok(serde_json::to_value(result).expect("ResultSet serialises"))
}

/// `GET/POST /v1/query/{name}[/{version}]` - run a stored query.
pub fn run_stored(
    dep: &Deployment,
    name: &str,
    version: Option<&str>,
    params: &Params,
) -> Result<Value, ApiError> {
    let aql = stored::get(dep.root(), name, version)?;
    run_aql(dep, &aql, params)
}

/// Validate a Composition against the RM and, optionally, a registered
/// template - without storing it. Returns the structured report so a caller
/// (e.g. an LLM agent over MCP) can self-correct. Unlike a commit, an invalid
/// Composition is not an error here: the report carries the verdict.
pub fn validate(
    dep: &Deployment,
    body: &str,
    template_id: Option<&str>,
) -> Result<Value, ApiError> {
    let composition: Composition = crate::rm::from_canonical_str(body)
        .map_err(|e| ApiError::BadRequest(format!("invalid Composition JSON: {e}")))?;
    let opt = match template_id {
        Some(id) => Some(
            dep.get_template(id)?
                .ok_or_else(|| ApiError::NotFound(format!("template `{id}` not registered")))?,
        ),
        None => None,
    };
    let report = crate::validate::validate(&composition, opt.as_ref());
    Ok(serde_json::to_value(report).expect("report serialises"))
}

/// `GET /v1/definition/template/adl1.4` - list registered template ids.
pub fn list_templates(dep: &Deployment) -> Result<Value, ApiError> {
    let ids = dep.list_templates()?;
    Ok(serde_json::json!({ "templates": ids }))
}

/// `GET /v1/definition/template/adl1.4/{id}` - fetch a template.
pub fn get_template(dep: &Deployment, id: &str) -> Result<Value, ApiError> {
    match dep.get_template(id)? {
        Some(opt) => {
            let json = opt
                .to_json()
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            read_json_str(&json)
        }
        None => Err(ApiError::NotFound(format!(
            "template `{id}` not registered"
        ))),
    }
}

/// Build a parameter map from a JSON `query_parameters` object (string-valued).
pub fn params_from_json(value: Option<&Value>) -> Params {
    let mut params = Params::new();
    if let Some(Value::Object(map)) = value {
        for (k, v) in map {
            let text = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            params.insert(k.clone(), text);
        }
    }
    params
}

fn current_version_uid(
    repo: &crate::store::EhrRepo,
    object_id: &str,
) -> Result<Option<String>, ApiError> {
    Ok(repo.log(object_id)?.first().map(|e| e.version_uid.clone()))
}

fn open_index(dep: &Deployment) -> Result<Index, ApiError> {
    Index::open(dep.root().join("index").join("aql.db")).map_err(ApiError::from)
}

fn read_json(path: &std::path::Path) -> Result<Value, ApiError> {
    let text = fs::read_to_string(path)
        .map_err(|e| ApiError::Internal(format!("reading {}: {e}", path.display())))?;
    read_json_str(&text)
}

fn read_json_str(text: &str) -> Result<Value, ApiError> {
    serde_json::from_str(text)
        .map_err(|e| ApiError::Internal(format!("malformed stored JSON: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A known RM-valid blood-pressure Composition claiming
    /// `vital_signs_encounter.v1` (shared with the rm crate's fixtures).
    const BP: &str = include_str!("../../tests/fixtures/blood-pressure-composition.json");

    fn fresh_deployment() -> (tempfile::TempDir, Deployment) {
        let tmp = tempfile::tempdir().unwrap();
        let dep = Deployment::init(
            tmp.path(),
            crate::store::DeploymentConfig::new("test.local"),
        )
        .unwrap();
        dep.install_starter_templates().unwrap();
        (tmp, dep)
    }

    fn ehr_id_of(ehr: &Value) -> String {
        ehr["ehr_id"]["value"].as_str().unwrap().to_string()
    }

    #[test]
    fn ehr_and_composition_lifecycle() {
        let (_tmp, dep) = fresh_deployment();
        let ehr_id = ehr_id_of(&create_ehr(&dep).unwrap());

        // Round-trip the EHR.
        assert_eq!(ehr_id_of(&get_ehr(&dep, &ehr_id).unwrap()), ehr_id);

        // Commit, then fetch by object id (head) and by version uid.
        let committed =
            commit_composition(&dep, &ehr_id, BP, None, None, &Committer::default()).unwrap();
        let object_id = committed.outcome.object_id.clone();
        let version_uid = committed.outcome.version_uid.clone();
        assert_eq!(
            get_composition(&dep, &ehr_id, &object_id).unwrap()["name"]["value"],
            "Blood pressure"
        );
        assert!(get_composition(&dep, &ehr_id, &version_uid).is_ok());

        // A stale If-Match is rejected for lost-update protection.
        let stale = commit_composition(
            &dep,
            &ehr_id,
            BP,
            Some(object_id),
            Some("\"wrong::x::1\""),
            &Committer::default(),
        );
        assert!(matches!(stale, Err(ApiError::PreconditionFailed(_))));
    }

    #[test]
    fn validate_reports_without_storing() {
        let (_tmp, dep) = fresh_deployment();
        let report = validate(&dep, BP, Some("vital_signs_encounter.v1")).unwrap();
        assert_eq!(report["valid"], true);
    }

    #[test]
    fn aql_runs_after_a_rest_commit_without_a_manual_index() {
        let (_tmp, dep) = fresh_deployment();
        let ehr_id = ehr_id_of(&create_ehr(&dep).unwrap());
        commit_composition(&dep, &ehr_id, BP, None, None, &Committer::default()).unwrap();

        // No explicit `index` step: run_aql refreshes incrementally.
        let result = run_aql(
            &dep,
            "SELECT COUNT(*) FROM COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]",
            &Params::new(),
        )
        .unwrap();
        assert_eq!(result["rows"][0][0], 1);
    }

    #[test]
    fn missing_ehr_is_not_found() {
        let (_tmp, dep) = fresh_deployment();
        assert!(matches!(get_ehr(&dep, "nope"), Err(ApiError::NotFound(_))));
    }
}
