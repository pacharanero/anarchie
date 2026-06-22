# anarchie serve

Serve the openEHR REST API over HTTP. The server binds to localhost and is built
on `tiny_http` - a plain blocking HTTP server, with no async runtime.

## Usage

```bash
anarchie serve [--addr host:port]
```

| Option          | Default            | Description                                  |
| --------------- | ------------------ | -------------------------------------------- |
| `--addr <addr>` | `127.0.0.1:8080`   | Address to bind, as `host:port`.             |

```bash
$ anarchie serve --addr 127.0.0.1:8137
anarchie REST API listening on http://127.0.0.1:8137 (Ctrl-C to stop)
```

## Endpoints

| Method        | Path                                          | What it does                                                              |
| ------------- | --------------------------------------------- | ------------------------------------------------------------------------ |
| `POST`        | `/v1/ehr`                                     | Create an EHR. `201` with a `Location` header.                           |
| `GET`         | `/v1/ehr/{id}`                                | Fetch an EHR.                                                             |
| `POST`        | `/v1/ehr/{id}/composition`                    | Commit a Composition. `201` with `ETag` (the `version_uid`) and `Location`. |
| `GET`         | `/v1/ehr/{id}/composition/{uid}`              | Fetch a Composition - a head object id or a full `obj::sys::N` version uid. |
| `PUT`         | `/v1/ehr/{id}/composition/{uid}`              | Commit a new version. Requires `If-Match`; `412` on a stale precondition. |
| `GET`/`POST`  | `/v1/query/aql`                               | Run an ad-hoc AQL query (`q` as a query parameter, or in the JSON body). |
| `GET`         | `/v1/query/{name}[/{version}]`                | Run a stored query.                                                       |
| `GET`         | `/v1/definition/template/adl1.4[/{id}]`       | List templates, or fetch one.                                            |

Validation failures return `422` with the same structured report you get from
[anarchie validate](validate.md).

## A read model that is always current

The query endpoints refresh the [index](index.md) incrementally before running -
only EHRs whose git `HEAD` moved are re-indexed. A Composition committed over
REST is therefore immediately queryable, with no separate `anarchie index` step.
(The CLI query commands do not do this; they read the index as it stands.)

## Example session

Start the server, then in another shell:

```bash
# Create an EHR (note the 201 and the Location header)
$ curl -s -i -X POST http://127.0.0.1:8137/v1/ehr
HTTP/1.1 201 Created
Server: tiny-http (Rust)
Content-Type: application/json
Location: /v1/ehr/6aeeffb0-0f9e-41be-be78-0e3d0d8028c1

{
  "_type": "EHR",
  "ehr_id": { "_type": "HIER_OBJECT_ID", "value": "6aeeffb0-0f9e-41be-be78-0e3d0d8028c1" },
  "system_id": { "_type": "HIER_OBJECT_ID", "value": "demo.local" },
  "time_created": { "value": "2026-06-22T12:07:25Z" }
}
```

```bash
# Commit a Composition into it (201, with the version_uid in the ETag)
$ EHR=6aeeffb0-0f9e-41be-be78-0e3d0d8028c1
$ curl -s -i -X POST "http://127.0.0.1:8137/v1/ehr/$EHR/composition" \
    -H "Content-Type: application/json" \
    --data-binary @blood-pressure-composition.json
HTTP/1.1 201 Created
Server: tiny-http (Rust)
Content-Type: application/json
ETag: "1e412356-46a6-4e94-9960-7eff11eb96f8::demo.local::1"
Location: /v1/ehr/6aeeffb0-0f9e-41be-be78-0e3d0d8028c1/composition/1e412356-46a6-4e94-9960-7eff11eb96f8::demo.local::1
```

```bash
# Query it straight away - the index refreshed itself first
$ curl -s -X POST http://127.0.0.1:8137/v1/query/aql \
    -H "Content-Type: application/json" \
    --data '{"q":"SELECT COUNT(*) FROM COMPOSITION c"}'
{
  "q": "SELECT COUNT(*) FROM COMPOSITION c",
  "columns": [
    {
      "name": "#count"
    }
  ],
  "rows": [
    [
      1
    ]
  ]
}
```

A Composition that breaches its template is rejected with `422` and the report:

```bash
$ curl -s -i -X POST "http://127.0.0.1:8137/v1/ehr/$EHR/composition" \
    -H "Content-Type: application/json" --data-binary @bad.json
HTTP/1.1 422 Unprocessable Entity
Content-Type: application/json

{
  "message": "Composition failed validation",
  "validation": {
    "valid": false,
    "violations": [
      {
        "severity": "error",
        "rm_path": "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude",
        "constraint": "C_DV_QUANTITY",
        "message": "magnitude 5000 outside permitted range for units \"mm[Hg]\""
      }
    ]
  }
}
```

A `PUT` whose `If-Match` does not match the current head is rejected with `412`,
so concurrent edits cannot silently clobber one another:

```bash
$ curl -s -i -X PUT "http://127.0.0.1:8137/v1/ehr/$EHR/composition/1e412356-46a6-4e94-9960-7eff11eb96f8" \
    -H "Content-Type: application/json" \
    -H 'If-Match: "1e412356-46a6-4e94-9960-7eff11eb96f8::demo.local::99"' \
    --data-binary @blood-pressure-composition.json
HTTP/1.1 412 Precondition Failed
Content-Type: application/json

{
  "message": "If-Match 1e412356-46a6-4e94-9960-7eff11eb96f8::demo.local::99 does not match current version Some(\"1e412356-46a6-4e94-9960-7eff11eb96f8::demo.local::1\")"
}
```

## See also

- [anarchie mcp](mcp.md) · [anarchie aql](aql.md) · [anarchie index](index.md)
- [Versioning and Git](../reference/versioning-and-git.md)
