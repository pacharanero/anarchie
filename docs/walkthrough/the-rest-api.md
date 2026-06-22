# The openEHR REST API

Everything so far has been the `anarchie` command line. But the whole point of a CDR is that *other software* talks to it - form renderers, clinical apps, integration engines. openEHR standardises that conversation as a [REST API](https://specifications.openehr.org/releases/ITS-REST/latest), and `anarchie serve` speaks it.

The same flat-file store is behind the API. There is no second copy of the data and no database - `serve` is a thin, stateless translation layer over the exact `ops` the CLI uses, built on the blocking `tiny_http` so there is still no async runtime and no extra dependency. The single-binary promise holds.

## Start the server

```bash
anarchie serve
```

```text title="output"
anarchie REST API listening on http://127.0.0.1:8080 (Ctrl-C to stop)
```

It binds to localhost only. Pass `--addr host:port` to change that. Leave it running and drive it from another shell with `curl`.

## Create an EHR

```bash
curl -i -X POST http://127.0.0.1:8080/v1/ehr
```

```text title="output"
HTTP/1.1 201 Created
Location: /v1/ehr/df171c82-9707-46e6-87f2-3716cd4e470e
Content-Type: application/json
…
```

## Commit a Composition

`POST …/composition` validates and stores, returning `201` with the new `version_uid` in the `ETag` and the canonical URL in `Location`:

```bash
curl -i -X POST http://127.0.0.1:8080/v1/ehr/$EHR/composition \
  -H 'Content-Type: application/json' \
  --data-binary @blood-pressure-composition.json
```

```text title="output"
HTTP/1.1 201 Created
ETag: "75953a2f-847e-4d15-9576-e78b7e5bb554::demo.openehr.org::1"
Location: /v1/ehr/df171c82-…/composition/75953a2f-…::demo.openehr.org::1
```

`GET …/composition/{uid}` reads it back, where `{uid}` is either the bare object id (the head) or a full `object::system::n` version uid (that exact version) - the same rule as `anarchie cat`.

## Optimistic concurrency with If-Match

openEHR uses HTTP preconditions for safe concurrent updates. To create a new version you `PUT` to the object id with an `If-Match` carrying the version you believe is current:

```bash
# succeeds: If-Match matches the current head, returns the new ::2 ETag
curl -i -X PUT http://127.0.0.1:8080/v1/ehr/$EHR/composition/$OBJECT_ID \
  -H 'If-Match: "75953a2f-…::demo.openehr.org::1"' \
  -H 'Content-Type: application/json' --data-binary @v2.json
```

```text title="output"
HTTP/1.1 200 OK
ETag: "75953a2f-847e-4d15-9576-e78b7e5bb554::demo.openehr.org::2"
```

If someone else committed in the meantime, your `If-Match` is stale and the write is refused - no lost update:

```bash
curl -o /dev/null -w "%{http_code}\n" -X PUT \
  http://127.0.0.1:8080/v1/ehr/$EHR/composition/$OBJECT_ID \
  -H 'If-Match: "75953a2f-…::demo.openehr.org::99"' \
  -H 'Content-Type: application/json' --data-binary @v2.json
```

```text title="output"
412
```

## Invalid data is refused at the boundary

The same native validator that guards `anarchie commit` guards the API. A nonconformant Composition is rejected with `422 Unprocessable Entity` and the structured violation report in the body - so a client knows exactly which path failed:

```bash
curl -o /dev/null -w "%{http_code}\n" -X POST \
  http://127.0.0.1:8080/v1/ehr/$EHR/composition \
  -H 'Content-Type: application/json' --data-binary @out-of-range.json
```

```text title="output"
422
```

## Query over HTTP

The AQL engine is exposed too. `GET /v1/query/aql?q=…` runs an ad-hoc query; `POST /v1/query/aql` takes a body with `query_parameters`; `GET /v1/query/{name}[/{version}]` runs a stored query.

```bash
curl --get http://127.0.0.1:8080/v1/query/aql \
  --data-urlencode "q=SELECT COUNT(*) FROM COMPOSITION c"
```

```json title="output"
{ "q": "SELECT COUNT(*) FROM COMPOSITION c", "columns": [ { "name": "#count" } ], "rows": [ [ 1 ] ] }
```

The query path refreshes the index incrementally *before* answering, so a Composition you just committed over REST is immediately queryable - there is no separate `index` step to run.

Template definitions are listed at `GET /v1/definition/template/adl1.4`, with an individual template at `…/adl1.4/{template_id}`.

!!! warning "A conformant subset, not a certified gateway"
    `anarchie serve` implements the EHR + Composition + AQL core of the openEHR REST API with the status codes, `ETag`/`If-Match` semantics, and ResultSet shapes a standard client expects. It is not a complete or certified implementation - the renderer formats (FLAT / STRUCTURED / Web Template) and the full `versioned_composition` surface are still ahead. See the [roadmap](../reference/roadmap.md).
