# The MCP server (openEHR for LLM agents)

The most forward-looking thing `anarchie` ships is `anarchie mcp` - a [Model Context Protocol](https://modelcontextprotocol.io/) server that exposes the CDR to LLM agents. MCP is the emerging standard for giving language models typed, discoverable tools; wiring `anarchie` up to it means an assistant in Claude Desktop, or any agent framework, can read and write a real, validated openEHR record.

It is the same store and the same `ops` layer as the CLI and the REST API - just a third front end. It speaks JSON-RPC 2.0 over stdio (stdin/stdout), which is exactly what MCP clients launch and talk to.

## Start it and list its tools

`anarchie mcp` reads JSON-RPC requests on stdin and writes responses on stdout. Here is a raw exchange - an `initialize` handshake followed by `tools/list` - piped straight in:

```bash
printf '%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | anarchie mcp
```

The `initialize` response identifies the server (`{"name":"anarchie","version":"0.0.0"}`), and `tools/list` returns the seven tools it offers:

| Tool | What it does |
|---|---|
| `create_ehr` | Create a new, empty EHR and return its `ehr.json` |
| `get_ehr` | Fetch an EHR by id |
| `get_composition` | Get a Composition by EHR id and uid (head object id or full version uid) |
| `commit_composition` | Validate and commit a Composition; pass `object_id` to add a new version |
| `validate_composition` | Validate against the RM and an optional template, without storing |
| `query_aql` | Run an AQL query and return the ResultSet |
| `list_templates` | List the registered template ids |

## Call a tool

`tools/call` invokes one by name. Listing the templates an agent can author against:

```bash
printf '%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_templates","arguments":{}}}' \
  | anarchie mcp
```

```json title="content of the tools/call result"
{
  "templates": [
    "adverse_reaction_list.v1",
    "problem_list.v1",
    "vital_signs_encounter.v1"
  ]
}
```

## Why this is the interesting part: validation as a feedback loop

The compelling move is what happens on a *bad* commit. When `commit_composition` is handed a nonconformant Composition, it does not just fail - it returns the structured validation report in-band, marked `isError: true`, naming the exact openEHR path that breached and why (for example, a systolic magnitude outside the permitted range for `mm[Hg]`).

That turns the validator into a **self-correction loop**. An LLM asked to author a blood-pressure Composition can commit a draft, read back the precise violation, fix that field, and retry - the same machine-readable feedback that makes `anarchie commit` strict is what lets an agent converge on conformant openEHR data without a human in the loop.

```text
agent: commit_composition(draft)      → isError: true, "…/value/magnitude outside range for mm[Hg]"
agent: (corrects the magnitude)
agent: commit_composition(fixed)      → committed, version_uid …::1
```

!!! tip "Wiring it into a client"
    Point any MCP client at the command `anarchie mcp`, run from inside a deployment directory. In Claude Desktop that is an entry in the MCP servers config; in an agent framework it is the stdio transport's launch command. From there the assistant can create patient records, author and validate Compositions, and answer AQL questions against the live store.
