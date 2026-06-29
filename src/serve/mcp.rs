// SPDX-License-Identifier: AGPL-3.0-or-later
//! A minimal stdio MCP (Model Context Protocol) server exposing the store to
//! LLM agents: get / commit / validate / query Compositions. The structured
//! validation report is surfaced verbatim so an agent can self-correct a
//! rejected Composition. See `specs/roadmap.md` (Phase 5).
//!
//! MCP's stdio transport is JSON-RPC 2.0 with newline-delimited messages: one
//! JSON object per line on stdin, one response per line on stdout. We implement
//! just `initialize`, `tools/list` and `tools/call` (plus `ping`), which is
//! enough for a client to discover and invoke the tools.

use std::io::{BufRead, Write};

use crate::store::Deployment;
use serde_json::{json, Value};

use crate::serve::ops::{self, ApiError, Committer};

const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the MCP server loop over stdin/stdout until EOF.
pub fn run(deployment: Deployment) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    eprintln!("anarchie MCP server on stdio (JSON-RPC; EOF to stop)");

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                write_message(&mut stdout, &parse_error(e.to_string()))?;
                continue;
            }
        };

        // Notifications (no `id`) get no response.
        let Some(id) = request.get("id").cloned() else {
            continue;
        };
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(Value::Null);

        let response = match dispatch(&deployment, method, &params) {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(rpc) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": rpc.0, "message": rpc.1 },
            }),
        };
        write_message(&mut stdout, &response)?;
    }
    Ok(())
}

/// A JSON-RPC error: `(code, message)`.
type RpcError = (i64, String);

fn dispatch(deployment: &Deployment, method: &str, params: &Value) -> Result<Value, RpcError> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "anarchie", "version": env!("CARGO_PKG_VERSION") },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_catalogue() })),
        "tools/call" => tools_call(deployment, params),
        other => Err((-32601, format!("method not found: {other}"))),
    }
}

/// Dispatch a `tools/call`, returning an MCP tool result (a `content` block).
fn tools_call(deployment: &Deployment, params: &Value) -> Result<Value, RpcError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| (-32602i64, "missing tool name".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let outcome = call_tool(deployment, name, &args);
    Ok(match outcome {
        Ok(value) => tool_result(&value, false),
        // A failed tool call is reported as an MCP tool error (not a protocol
        // error), so the agent receives the message and can adjust.
        Err(err) => tool_result(&api_error_to_json(err), true),
    })
}

fn call_tool(deployment: &Deployment, name: &str, args: &Value) -> Result<Value, ApiError> {
    let str_arg = |key: &str| args.get(key).and_then(Value::as_str).map(str::to_string);
    let required = |key: &str| {
        str_arg(key).ok_or_else(|| ApiError::BadRequest(format!("missing argument `{key}`")))
    };
    // Composition arguments may be passed as a JSON object or a JSON string.
    let composition_body = |key: &str| -> Result<String, ApiError> {
        match args.get(key) {
            Some(Value::String(s)) => Ok(s.clone()),
            Some(other) => Ok(other.to_string()),
            None => Err(ApiError::BadRequest(format!("missing argument `{key}`"))),
        }
    };

    match name {
        "create_ehr" => ops::create_ehr(deployment),
        "get_ehr" => ops::get_ehr(deployment, &required("ehr_id")?),
        "get_composition" => {
            ops::get_composition(deployment, &required("ehr_id")?, &required("uid")?)
        }
        "commit_composition" => {
            let committed = ops::commit_composition(
                deployment,
                &required("ehr_id")?,
                &composition_body("composition")?,
                str_arg("object_id"),
                None,
                &Committer::default(),
            )?;
            Ok(json!({
                "version_uid": committed.outcome.version_uid,
                "object_id": committed.outcome.object_id,
                "commit": committed.outcome.commit_sha,
            }))
        }
        "validate_composition" => ops::validate(
            deployment,
            &composition_body("composition")?,
            str_arg("template_id").as_deref(),
        ),
        "query_aql" => {
            let params = ops::params_from_json(args.get("query_parameters"));
            ops::run_aql(deployment, &required("query")?, &params)
        }
        "list_templates" => ops::list_templates(deployment),
        other => Err(ApiError::NotFound(format!("unknown tool `{other}`"))),
    }
}

/// Wrap a JSON value as an MCP `tools/call` result with a single text block.
fn tool_result(value: &Value, is_error: bool) -> Value {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

fn api_error_to_json(err: ApiError) -> Value {
    match err {
        ApiError::Validation(report) => json!({
            "error": "validation failed",
            "validation": report,
        }),
        ApiError::BadRequest(m) => json!({ "error": m }),
        ApiError::NotFound(m) => json!({ "error": m }),
        ApiError::PreconditionFailed(m) => json!({ "error": m }),
        ApiError::Internal(m) => json!({ "error": m }),
    }
}

fn write_message(out: &mut impl Write, message: &Value) -> std::io::Result<()> {
    let line = serde_json::to_string(message).unwrap_or_else(|_| "{}".into());
    out.write_all(line.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()
}

fn parse_error(detail: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": Value::Null,
        "error": { "code": -32700, "message": format!("parse error: {detail}") },
    })
}

/// The tool catalogue advertised to MCP clients.
fn tool_catalogue() -> Value {
    let obj = |props: Value, required: Value| json!({ "type": "object", "properties": props, "required": required });
    let string = json!({ "type": "string" });
    json!([
        {
            "name": "create_ehr",
            "description": "Create a new, empty EHR and return its ehr.json.",
            "inputSchema": obj(json!({}), json!([])),
        },
        {
            "name": "get_ehr",
            "description": "Fetch an EHR by id.",
            "inputSchema": obj(json!({ "ehr_id": string }), json!(["ehr_id"])),
        },
        {
            "name": "get_composition",
            "description": "Get a Composition by EHR id and uid (a head object id or a full version uid).",
            "inputSchema": obj(
                json!({ "ehr_id": string, "uid": string }),
                json!(["ehr_id", "uid"]),
            ),
        },
        {
            "name": "commit_composition",
            "description": "Validate and commit a Composition into an EHR. Pass object_id to add a new version. On validation failure the structured report is returned so you can correct and retry.",
            "inputSchema": obj(
                json!({
                    "ehr_id": string,
                    "composition": { "type": ["object", "string"] },
                    "object_id": string,
                }),
                json!(["ehr_id", "composition"]),
            ),
        },
        {
            "name": "validate_composition",
            "description": "Validate a Composition against the RM and, optionally, a registered template, without storing it. Returns the structured violation report.",
            "inputSchema": obj(
                json!({
                    "composition": { "type": ["object", "string"] },
                    "template_id": string,
                }),
                json!(["composition"]),
            ),
        },
        {
            "name": "query_aql",
            "description": "Run an ad-hoc AQL query against the index and return an openEHR ResultSet.",
            "inputSchema": obj(
                json!({ "query": string, "query_parameters": { "type": "object" } }),
                json!(["query"]),
            ),
        },
        {
            "name": "list_templates",
            "description": "List the registered Operational Template ids.",
            "inputSchema": obj(json!({}), json!([])),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deployment() -> (tempfile::TempDir, Deployment) {
        let tmp = tempfile::tempdir().unwrap();
        let dep = Deployment::init(tmp.path(), crate::store::DeploymentConfig::new("t")).unwrap();
        (tmp, dep)
    }

    #[test]
    fn initialize_and_tools_list_are_well_formed() {
        let (_tmp, dep) = deployment();
        let init = dispatch(&dep, "initialize", &json!({})).unwrap();
        assert_eq!(init["protocolVersion"], PROTOCOL_VERSION);

        let tools = dispatch(&dep, "tools/list", &json!({})).unwrap();
        let names: Vec<&str> = tools["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"commit_composition"));
        assert!(names.contains(&"query_aql"));
    }

    #[test]
    fn unknown_method_is_a_jsonrpc_error() {
        let (_tmp, dep) = deployment();
        let err = dispatch(&dep, "no_such_method", &json!({})).unwrap_err();
        assert_eq!(err.0, -32601);
    }

    #[test]
    fn tool_error_is_reported_in_band() {
        let (_tmp, dep) = deployment();
        let result = dispatch(
            &dep,
            "tools/call",
            &json!({ "name": "get_ehr", "arguments": { "ehr_id": "missing" } }),
        )
        .unwrap();
        // A missing EHR is an in-band tool error, not a protocol error.
        assert_eq!(result["isError"], true);
    }
}
