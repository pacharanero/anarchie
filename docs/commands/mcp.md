# anarchie mcp

Run a stdio **Model Context Protocol** server, exposing the store to LLM agents.
It speaks JSON-RPC 2.0 over stdin/stdout, so it slots straight into LLM agent
frameworks and desktop assistants that launch MCP servers as subprocesses.

## Usage

```bash
anarchie mcp
```

The server reads one JSON-RPC request per line on stdin and writes one response
per line on stdout, until end-of-file. It implements the MCP methods
`initialize`, `tools/list` and `tools/call`.

## Tools

It exposes seven tools - the store's capabilities, in agent-callable form:

| Tool                   | What it does                                                       |
| ---------------------- | ----------------------------------------------------------------- |
| `create_ehr`           | Create a new, empty EHR and return its `ehr.json`.                |
| `get_ehr`              | Fetch an EHR by id.                                                |
| `get_composition`      | Fetch a Composition by EHR id and uid (head object id or version uid). |
| `commit_composition`   | Validate and commit a Composition; pass `object_id` to add a version. |
| `validate_composition` | Validate a Composition against the RM and, optionally, a template. |
| `query_aql`            | Run an ad-hoc AQL query and return an openEHR ResultSet.          |
| `list_templates`       | List the registered Operational Template ids.                     |

## Agents can self-correct

When `commit_composition` rejects a Composition, the structured validation report
comes back **in-band** as the tool result with `isError: true`, rather than as a
transport-level error. The agent sees exactly which constraints were breached,
can amend the Composition, and retry - a tight write-validate-fix loop without a
human in the middle.

## Example exchange

Pipe two JSON-RPC lines - an `initialize` then a `tools/list` - into the server.
The startup banner is written to stderr; the JSON-RPC responses go to stdout:

```bash
$ printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0.1"}}}' \
    '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | anarchie mcp
anarchie MCP server on stdio (JSON-RPC; EOF to stop)
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"anarchie","version":"0.0.0"}}}
{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"create_ehr",...},{"name":"get_ehr",...},{"name":"get_composition",...},{"name":"commit_composition",...},{"name":"validate_composition",...},{"name":"query_aql",...},{"name":"list_templates",...}]}}
```

Each entry in the `tools/list` result carries a `description` and a JSON-Schema
`inputSchema`, so an LLM client can call the tools without any out-of-band
knowledge of the store.

## See also

- [anarchie serve](serve.md) · [anarchie validate](validate.md) · [anarchie aql](aql.md)
- [Roadmap](../reference/roadmap.md)
