// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! A small openEHR REST API over the store, using the blocking `tiny_http`
//! server (no async runtime - keeping the single-binary, dependency-light
//! promise). It is a thin translation onto [`crate::serve::ops`]; it owns no data.
//!
//! Conformance is to a documented subset (see `specs/rest-api.md`): EHR and
//! Composition core, ad-hoc and stored AQL, and template read. Auth and
//! multi-tenancy are out of scope - the server binds to localhost and assumes a
//! trusted operator.

use crate::query::Params;
use crate::store::Deployment;
use serde_json::Value;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::serve::ops::{self, ApiError, Committer};

/// Start the REST server on `addr` (blocking) against the deployment at `root`.
pub fn serve(deployment: Deployment, addr: &str) -> std::io::Result<()> {
    let server = Server::http(addr).map_err(std::io::Error::other)?;
    eprintln!("anarchie REST API listening on http://{addr} (Ctrl-C to stop)");
    for request in server.incoming_requests() {
        handle(&deployment, request);
    }
    Ok(())
}

/// A handler outcome: an HTTP status, a JSON body, and extra response headers.
struct Reply {
    status: u16,
    body: Value,
    headers: Vec<(String, String)>,
}

impl Reply {
    fn new(status: u16, body: Value) -> Self {
        Self {
            status,
            body,
            headers: Vec::new(),
        }
    }

    fn header(mut self, name: &str, value: String) -> Self {
        self.headers.push((name.to_string(), value));
        self
    }
}

fn handle(deployment: &Deployment, mut request: Request) {
    let reply = route(deployment, &mut request).unwrap_or_else(error_reply);

    let data = serde_json::to_string_pretty(&reply.body).unwrap_or_else(|_| "{}".into());
    let mut response = Response::from_string(data).with_status_code(reply.status);
    response.add_header(json_header());
    for (name, value) in &reply.headers {
        if let Ok(h) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response.add_header(h);
        }
    }
    let _ = request.respond(response);
}

/// Route a request to a handler, reading the body where needed.
fn route(deployment: &Deployment, request: &mut Request) -> Result<Reply, ApiError> {
    let method = request.method().clone();
    let url = request.url().to_string();
    let (path, query) = split_query(&url);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    match (&method, segments.as_slice()) {
        // EHR core
        (Method::Post, ["v1", "ehr"]) => {
            let ehr = ops::create_ehr(deployment)?;
            let id = ehr
                .get("ehr_id")
                .and_then(|u| u.get("value"))
                .and_then(Value::as_str);
            let mut reply = Reply::new(201, ehr.clone());
            if let Some(id) = id {
                reply = reply.header("Location", format!("/v1/ehr/{id}"));
            }
            Ok(reply)
        }
        (Method::Get, ["v1", "ehr", ehr_id]) => {
            Ok(Reply::new(200, ops::get_ehr(deployment, ehr_id)?))
        }

        // Composition core
        (Method::Post, ["v1", "ehr", ehr_id, "composition"]) => {
            let body = read_body(request)?;
            let committed = ops::commit_composition(
                deployment,
                ehr_id,
                &body,
                None,
                None,
                &Committer::default(),
            )?;
            Ok(commit_reply(201, ehr_id, committed))
        }
        (Method::Get, ["v1", "ehr", ehr_id, "composition", uid]) => Ok(Reply::new(
            200,
            ops::get_composition(deployment, ehr_id, uid)?,
        )),
        (Method::Put, ["v1", "ehr", ehr_id, "composition", uid]) => {
            let body = read_body(request)?;
            let if_match = header_value(request, "If-Match");
            let committed = ops::commit_composition(
                deployment,
                ehr_id,
                &body,
                Some((*uid).to_string()),
                if_match.as_deref(),
                &Committer::default(),
            )?;
            Ok(commit_reply(200, ehr_id, committed))
        }

        // Query — ad-hoc AQL (note: matched before the stored-query catch-all)
        (Method::Get, ["v1", "query", "aql"]) => {
            let q = query_param(query, "q").ok_or_else(|| {
                ApiError::BadRequest("missing required `q` query parameter".into())
            })?;
            Ok(Reply::new(
                200,
                ops::run_aql(deployment, &q, &Params::new())?,
            ))
        }
        (Method::Post, ["v1", "query", "aql"]) => {
            let body = read_body(request)?;
            let json: Value = serde_json::from_str(&body)
                .map_err(|e| ApiError::BadRequest(format!("invalid request body: {e}")))?;
            let q = json
                .get("q")
                .and_then(Value::as_str)
                .ok_or_else(|| ApiError::BadRequest("body must contain `q`".into()))?;
            let params = ops::params_from_json(json.get("query_parameters"));
            Ok(Reply::new(200, ops::run_aql(deployment, q, &params)?))
        }

        // Query — stored (named) queries
        (Method::Get, ["v1", "query", name]) => Ok(Reply::new(
            200,
            ops::run_stored(deployment, name, None, &Params::new())?,
        )),
        (Method::Get, ["v1", "query", name, version]) => Ok(Reply::new(
            200,
            ops::run_stored(deployment, name, Some(version), &Params::new())?,
        )),

        // Definition — templates
        (Method::Get, ["v1", "definition", "template", "adl1.4"]) => {
            Ok(Reply::new(200, ops::list_templates(deployment)?))
        }
        (Method::Get, ["v1", "definition", "template", "adl1.4", id]) => {
            Ok(Reply::new(200, ops::get_template(deployment, id)?))
        }

        _ => Err(ApiError::NotFound(format!("no route for {method} {path}"))),
    }
}

fn commit_reply(status: u16, ehr_id: &str, committed: crate::serve::ops::Committed) -> Reply {
    let uid = committed.outcome.version_uid.clone();
    Reply::new(status, committed.composition)
        .header("ETag", format!("\"{uid}\""))
        .header("Location", format!("/v1/ehr/{ehr_id}/composition/{uid}"))
}

/// Map an [`ApiError`] to a status code and JSON error body.
fn error_reply(err: ApiError) -> Reply {
    match err {
        ApiError::BadRequest(m) => Reply::new(400, message(&m)),
        ApiError::NotFound(m) => Reply::new(404, message(&m)),
        ApiError::PreconditionFailed(m) => Reply::new(412, message(&m)),
        ApiError::Validation(report) => Reply::new(
            422,
            serde_json::json!({
                "message": "Composition failed validation",
                "validation": report,
            }),
        ),
        ApiError::Internal(m) => Reply::new(500, message(&m)),
    }
}

fn message(text: &str) -> Value {
    serde_json::json!({ "message": text })
}

fn json_header() -> Header {
    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("static header is valid")
}

fn read_body(request: &mut Request) -> Result<String, ApiError> {
    let mut body = String::new();
    request
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|e| ApiError::BadRequest(format!("could not read request body: {e}")))?;
    Ok(body)
}

fn header_value(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str().to_string())
}

/// Split a URL into its path and (optional) raw query string.
fn split_query(url: &str) -> (&str, Option<&str>) {
    match url.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (url, None),
    }
}

/// Find and percent-decode a query parameter from a raw query string.
fn query_param(query: Option<&str>, key: &str) -> Option<String> {
    let query = query?;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == key {
            return Some(percent_decode(v));
        }
    }
    None
}

/// Decode `application/x-www-form-urlencoded` text: `+` to space and `%XX`
/// escapes to bytes.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as u8);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_percent_and_plus() {
        assert_eq!(percent_decode("a%20b+c"), "a b c");
        assert_eq!(percent_decode("o%2Fdata%5Bat0001%5D"), "o/data[at0001]");
    }

    #[test]
    fn splits_and_finds_query_params() {
        let (path, query) = split_query("/v1/query/aql?q=SELECT%201&x=2");
        assert_eq!(path, "/v1/query/aql");
        assert_eq!(query_param(query, "q").as_deref(), Some("SELECT 1"));
        assert_eq!(query_param(query, "x").as_deref(), Some("2"));
        assert_eq!(query_param(query, "missing"), None);
    }
}
