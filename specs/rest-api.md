# REST API

The openEHR Foundation publishes a normative [REST API specification](https://specifications.openehr.org/releases/ITS-REST/latest). Conforming to it - rather than inventing a bespoke API - is what lets existing openEHR applications, form renderers, and tools talk to `anarchie` unchanged. This is the same discipline as `sct` implementing the FHIR terminology operations (`$lookup`, `$validate-code`, `$expand`) rather than a custom search API.

The REST server is an **outer layer** of the onion (`anarchie serve`), optional and stateless over the store. Like `sct serve`, it is a feature-gated subcommand so the core tool stays dependency-light.

---

## Conformance posture

`anarchie` aims for **conformance to a documented subset**, honestly labelled. A partial-but-correct implementation is more useful than a complete-but-wrong one. The subset is published so clients know what to expect, and responses use the openEHR-specified status codes and error bodies.

---

## API surface (phased)

### Phase 1 — EHR + Composition core

| Method | Path | openEHR operation |
|---|---|---|
| `POST` | `/v1/ehr` | Create EHR |
| `GET` | `/v1/ehr/{ehr_id}` | Get EHR |
| `GET` | `/v1/ehr?subject_id=…` | Get EHR by subject |
| `PUT` | `/v1/ehr/{ehr_id}/ehr_status` | Update EHR_STATUS (versioned) |
| `GET` | `/v1/ehr/{ehr_id}/ehr_status` | Get EHR_STATUS |
| `POST` | `/v1/ehr/{ehr_id}/composition` | Commit a Composition (→ 1 git commit) |
| `GET` | `/v1/ehr/{ehr_id}/composition/{uid}` | Get Composition (versioned uid or object id) |
| `PUT` | `/v1/ehr/{ehr_id}/composition/{uid}` | New version of a Composition |
| `DELETE` | `/v1/ehr/{ehr_id}/composition/{uid}` | Logically delete (a new "deleted" version) |

### Phase 2 — Query

| Method | Path | openEHR operation |
|---|---|---|
| `GET` | `/v1/query/aql?q=…` | Execute AQL (ad-hoc) |
| `POST` | `/v1/query/aql` | Execute AQL (body, with parameters) |

### Phase 3 — Definition / templates + advanced

| Method | Path | openEHR operation |
|---|---|---|
| `POST` | `/v1/definition/template/adl1.4` | Upload an Operational Template |
| `GET` | `/v1/definition/template/adl1.4` | List templates |
| `GET` | `/v1/definition/template/adl1.4/{id}` | Get a template |
| `GET` | `/v1/ehr/{ehr_id}/versioned_composition/{id}/version` | Version history |
| `GET` | `/v1/ehr/{ehr_id}/versioned_composition/{id}/version/{time}` | Version at time |
| `GET` | `/v1/ehr/{ehr_id}/contribution/{id}` | Get a contribution |

---

## How REST operations map to the store

The server is a thin translation from HTTP to store operations - it owns no data:

- **Commit Composition** → validate (RM + OPT) → assign `version_uid` → write canonical JSON to the working tree → `git commit` (= the `CONTRIBUTION`) → update/dirty the index → return `201` with `ETag: <version_uid>` and `Location`.
- **Get Composition** → working-tree read (head) or `git show` (historical version).
- **Update Composition** → enforce `If-Match` precondition against current head version (optimistic concurrency); mismatch → `412 Precondition Failed`.
- **AQL** → hand the query string to the query engine, which hits the derived index and returns an openEHR `ResultSet`.
- **Version at time** → resolve to the git commit at-or-before the timestamp, then `git show`.

---

## Headers and conformance details that matter

- **`ETag` / `If-Match`** - carry the `version_uid`; this is how openEHR does optimistic locking and how `anarchie` enforces lost-update protection (see [versioning-and-git.md](versioning-and-git.md)).
- **`Prefer: return=representation|minimal`** - whether the body echoes the stored object.
- **`Location`** - the canonical URL of a newly created resource.
- **Content types** - `application/json` for canonical JSON. (XML/`application/xml` is a later, optional concern; canonical JSON is the on-disk and on-the-wire default.)
- **Status codes** - `201` create, `200` get/update, `204` delete, `400` malformed, `404` not found, `409` conflict, `412` precondition failed, `422` validation failure (with the structured violation list from [validation.md](validation.md)).

---

## Auth and multi-tenancy

Out of scope for the MVP. `anarchie serve` binds to localhost and assumes a trusted single operator, exactly as `sct gui`/`sct serve` do. Production concerns (OAuth, per-EHR access control, audit of *reads*) are deferred; the repo-per-EHR topology at least gives a natural future boundary for access control.

---

## Why bother being conformant?

The entire value proposition rests on `anarchie` being a *drop-in-ish* openEHR CDR. An openEHR form renderer, an EHRbase-targeting app, or a test harness should be able to point at `anarchie serve` and mostly work. Conformance is what turns "a clever way to store JSON in git" into "a CDR you can actually use with the existing ecosystem" - the same way `sct` speaking FHIR turns a local SNOMED file into something Ontoserver-compatible clients can query.
