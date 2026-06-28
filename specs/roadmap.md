# Roadmap

A phased path from specification to a usable, experimental openEHR CDR. `anarchie` is primarily a **learning and experimentation project**, so the roadmap optimises for *learning something at each step* and for *always having a working artefact*, rather than for racing to feature-completeness.

Each phase produces something runnable and inspectable. No phase depends on a later phase to be useful.

---

## Phase 0 — Specification (current)

**Goal:** decide whether the idea is sound before writing code.

- [x] README and pitch
- [x] Architecture / onion model
- [x] On-disk format
- [x] Versioning-and-git design (decision: git is intrinsic)
- [x] Validation strategy (decision: Rust-native)
- [x] Query engine approach
- [x] REST API surface
- [x] Serialisation formats (canonical JSON / XML / FLAT / STRUCTURED / Web Template)
- [x] Reference Model coverage tracker
- [x] openEHR terminology code reference (change-type, lifecycle, category, ISM)
- [x] Scaling envelope (honest limits)
- [ ] Circulate for critique (openEHR Discourse, gitehr overlap)

**Exit criterion:** the open questions in [architecture.md](architecture.md) are resolved or consciously deferred.

---

## Phase 1 — The Reference Model in Rust

**Goal:** represent and round-trip openEHR data faithfully. Nothing else works without this.

- [x] `anarchie-rm`: Rust types for the core RM (`COMPOSITION`, `SECTION`, `OBSERVATION`, `EVALUATION`, `INSTRUCTION`, `ACTION`, `CLUSTER`, `ELEMENT`, the `DV_*` data values).
- [x] `serde` (de)serialisation to/from openEHR **canonical JSON**.
- [x] **Byte-stable canonical serialisation** - the hard dependency; prove a round-trip is idempotent and diff-friendly.
- [x] `anarchie info <composition.json>` - inspect any Composition file.

**Learning milestone:** can we represent real Compositions (from the CKM / EHRbase examples) losslessly in Rust? *Answered in part: a realistic blood-pressure Composition round-trips, and canonical serialisation is proven idempotent (the byte-stability question from [architecture.md](architecture.md)). Known gap: a standalone `DV_IDENTIFIER` inside `PARTY_IDENTIFIED.identifiers` does not yet re-emit its `_type`.*

---

## Phase 2 — The file store + git

**Goal:** a durable, versioned, inspectable store - without validation or query yet.

- [x] `anarchie init` - scaffold a deployment.
- [x] Repo-per-EHR layout, working-tree-holds-head convention.
- [x] `anarchie commit <comp.json>` - write canonical JSON + `git commit` as a `CONTRIBUTION`.
- [x] `version_uid` assignment and the contribution manifest.
- [x] `anarchie cat`, `anarchie log` (head via filesystem, history via git).
- [x] `anarchie diff v1 v2` - git diff + a structural layer.

**Learning milestone:** does the CONTRIBUTION-as-commit mapping feel natural in practice, and does git stay legible? *Yes - the commit graph reads cleanly: each contribution is one commit carrying `anarchie-contribution-id` / `anarchie-change-type` / `anarchie-system-id` trailers, `git log -- <path>` is the version history, and because canonical JSON is byte-stable a re-commit of identical content diffs to just the `version_uid` bump. The `anarchie-store` crate shells out to the system `git` (no libgit2), keeping the binary light and the repo an ordinary git repo. Open question deferred to Phase 4: the contribution manifest omits its own commit sha (a commit cannot contain its own hash); the contribution-to-commit link is the trailer, resolved at read time.*


---

## Phase 3 — Validation

**Goal:** reject invalid data at the door; become a real CDR rather than a JSON folder.

- [x] `anarchie-aom` - constraint types (Archetype Object Model).
- [x] `anarchie-opt` - parse a flattened Operational Template into an AOM tree.
- [x] `anarchie template add` - register templates as the schema.
- [x] `anarchie-validate` - RM + OPT tree-walk producing structured violations.
- [x] Wire validation into the commit path (invalid → rejected).
- [ ] Ingest real `.opt` XML (Archetype Designer / ADL Workbench export) into the AOM tree, in addition to anarchie's native flattened-JSON form (see [serialisation-formats.md](serialisation-formats.md)).
- [ ] Cross-check harness against Archie (JVM as a *test-time* oracle only). *Deferred to a later iteration: it needs the JVM toolchain and a curated conformance corpus, and is independent of the validator's own design.*

**Learning milestone:** can a pure-Rust validator agree with Archie on the conformance test corpus? This is the project's biggest risk and biggest learning. *Partly answered, partly deferred. The architecture that emerged: RM validation walks the **typed** Reference Model tree (`anarchie-rm` structs) checking invariants that hold for every Composition (CODE_PHRASE completeness, ELEMENT value-XOR-null_flavour, DV_QUANTITY `magnitude_status`, DV_PROPORTION kind/denominator), while OPT validation walks the **canonical JSON** guided by the AOM constraint tree - matching `C_COMPLEX_OBJECT` children by `archetype_node_id`, enforcing occurrences / existence / cardinality, and applying leaf constraints (`C_DV_QUANTITY` units + magnitude range, `C_CODE_PHRASE` terminology + code set, `C_STRING` value list, `C_DV_ORDINAL`). The key insight: the AOM names RM attributes as **strings** that map directly onto JSON keys, so the OPT walk over `serde_json::Value` is dramatically simpler than trying to reflect over typed enums - the typed tree is right for universal invariants, the JSON tree is right for archetype-specific constraints. The Archie cross-check (the actual corpus-agreement question) is deferred; what is proven now is that the validator catches real breaches end-to-end (an out-of-range systolic is rejected at `anarchie commit` with a precise openEHR path) and that valid data round-trips clean. Operational Templates are anarchie's own native flattened-JSON form for now; ingesting `.opt` XML from Archetype Designer is future work.*

---

## Phase 3.5 — Batteries-included starter templates

**Goal:** `anarchie init` yields a CDR that can store real clinical data immediately, not an empty repo.

- [x] Curate the Tier 1 template set from openEHR International archetypes (see [bundled-archetypes.md](bundled-archetypes.md)). *The full Tier-1 IPS span, eight templates: `vital_signs_encounter` (blood pressure, pulse, temperature, respiration, weight, height), `problem_list` (`EVALUATION.problem_diagnosis`), `adverse_reaction_list` (`EVALUATION.adverse_reaction_risk`), `medication_list` (`OBSERVATION.medication_statement.v0`, v0/draft), `laboratory_result_report` (`OBSERVATION.laboratory_test_result.v1`), `immunisation_list` (`ACTION.medication.v1`), `procedure_list` (`ACTION.procedure.v1`), and `encounter_note` (`EVALUATION.clinical_synopsis.v1`) - covering all three required IPS sections (problems, allergies, medications) plus the recommended ones. See [ips-readiness.md](ips-readiness.md).*
- [x] Build the OPTs as anarchie's native flattened OPT JSON. *Hand-authored against the real archetype at-codes for the MVP; the build-time flatten via Archetype Designer / ADL Workbench / Archie remains the path once `.opt` XML ingest lands (Phase 3 open item).*
- [x] Dual-license: code **AGPL-3.0-or-later**, bundled OPTs under **CC-BY-SA 3.0** with a provenance manifest (see [licensing.md](licensing.md)). *`crates/anarchie-store/src/starter/templates/attribution.md` records per-template provenance and the ShareAlike notice, embedded in the binary and written into each deployment's `templates/` alongside the installed models so the licence travels with the data.*
- [x] `anarchie init` installs the starter set by default; `--minimal` yields an empty CDR.
- [ ] Verify CKM Terms of Use wording and quote it in the bundle attribution before shipping. *Packaging-time step; deferred until a release is cut.*

**Learning milestone:** can a newcomer store a believable patient record (vitals, problems, meds, allergies, results) within five minutes of install? *Demonstrated end-to-end for the bundled span: a fresh `anarchie init` ships three registered templates, and a real blood-pressure Composition validates against `vital_signs_encounter.v1` and commits in one step with no authoring. The templates are embedded with `include_str!`, so the single-binary promise holds - no companion files needed at runtime.*

---

## Phase 4 — Query (AQL)

**Goal:** answer population queries, not just id lookups.

- [x] `anarchie index` - flatten Compositions into a path-value table in SQLite (`anarchie-query`, bundled `rusqlite`, no runtime dependency).
- [x] Index freshness tracking; `--rebuild` backstop. *The index stores each EHR's last-indexed git HEAD in an `ehr_freshness` table; a plain `anarchie index` re-indexes only EHRs whose HEAD moved, and `--rebuild` drops and rebuilds the lot. Freshness lives in the derived index, not the manifest, keeping the read model self-describing and disposable.*
- [x] AQL parser (hand-written Rust lexer + recursive-descent parser) for the MVP subset - `SELECT` of leaf paths and aggregates, `FROM … CONTAINS …`, `WHERE` (comparisons, `MATCHES`, `LIKE`, `EXISTS`, `AND`/`OR`/`NOT`), `ORDER BY`, `LIMIT`/`OFFSET`, `$`-parameters.
- [x] AQL → SQL translation over the path index. *The index keys every leaf by its composition-rooted canonical path and tags each row with its ENTRY archetype, so an identified path resolves to an exact `path` lookup and `CONTAINS OBSERVATION o[id]` to an exact `entry_archetype` match - no JSON walking at query time. Comparisons/aggregates become correlated `EXISTS`/sub-selects over `path_value`.*
- [x] `anarchie aql "SELECT …"` returning an openEHR-style ResultSet (`{q, columns, rows}`).
- [x] Stored (named) queries registered as git-versioned data, executed by name and version. *`anarchie query add|list|run`; queries live as `queries/<name>/<version>.aql` files alongside templates - data, not code.*
- [ ] DuckDB/Parquet path for aggregate analytics (explore how much it gives for free). *Deferred: the SQLite path covers the MVP aggregates (`COUNT`/`MIN`/`MAX`/`SUM`/`AVG`); the columnar analytics engine is a later, additive exploration.*

**Learning milestone:** how much of AQL can DuckDB-over-JSON handle before a bespoke engine is unavoidable? *Reframed by what we built: the SQLite path-extraction index answers the MVP AQL subset directly - the "flatten once, serve many" move from `sct` - proven end-to-end (a real blood-pressure Composition is indexed and a `WHERE systolic > 140` / `COUNT(*)` / `AVG` / parameterised query returns the right rows via `anarchie index` + `anarchie aql`). The DuckDB-over-JSON question is still open and now isolated to the analytics tier, not the core retrieval path.*

---

## Phase 5 — Services (REST + MCP)

**Goal:** interoperate with the existing openEHR ecosystem and with AI agents.

- [x] `anarchie serve` - openEHR REST API (Phase 1 surface from [rest-api.md](rest-api.md)): EHR + Composition CRUD, `If-Match` concurrency. *Built on the blocking `tiny_http` (no async runtime - the single-binary promise holds). `POST /v1/ehr`, `GET /v1/ehr/{id}`, `POST`/`GET`/`PUT …/composition` with `ETag`/`Location` on writes and a `412` on a stale `If-Match`; validation failures surface as `422` with the structured report. The new `anarchie-serve` crate is a thin, data-less translation onto a shared `ops` layer.*
- [x] AQL endpoint (ad-hoc + stored queries). *`GET /v1/query/aql?q=`, `POST /v1/query/aql` (with `query_parameters`), and `GET /v1/query/{name}[/{version}]`. The query path refreshes the index incrementally first, so a Composition committed over REST is queryable with no separate `index` step.*
- [x] Template definition endpoints (`GET /v1/definition/template/adl1.4` list + `…/{id}` get). *Example-Composition generation from a template is deferred - it depends on Web Template generation below.*
- [ ] Web Template generation on template registration; FLAT / STRUCTURED conversion at the REST boundary (see [serialisation-formats.md](serialisation-formats.md)). *Deferred: the renderer-format conversions are a self-contained serialisation workstream; the store and wire format stay canonical JSON for now.*
- [ ] Cross-check REST/AQL behaviour against the EHRbase sandbox as a test-time oracle. *Deferred (external oracle), like the Archie cross-check in Phase 3.*
- [x] `anarchie mcp` - stdio MCP server: get/commit/validate/query Compositions for LLM agents, reusing the structured violation output to let an agent self-correct. *JSON-RPC 2.0 over stdio (`initialize`/`tools/list`/`tools/call`); seven tools over the same `ops` layer. A rejected commit returns the validation report in-band (`isError: true`) so the agent can fix and retry.*

**Learning milestone:** can an existing openEHR app or form renderer point at `anarchie serve` and work? *Partly answered: the EHR/Composition/AQL core is conformant enough that a client speaking the openEHR REST shapes gets the expected status codes, `ETag`/`If-Match` optimistic concurrency, and an openEHR-style AQL ResultSet - demonstrated end-to-end with curl (create EHR → commit → versioned PUT with If-Match → ad-hoc & stored AQL). The renderer formats (FLAT/STRUCTURED/Web Template) a form renderer would also want are the deferred piece, and the EHRbase cross-check remains the honest next step to quantify "mostly works".*

---

## Phase 6 — Integration and convergence (speculative)

**Goal:** explore the interesting collisions.

This phase is **speculative**: most items are integrations with external systems or research questions, deliberately left open. The two self-contained engineering items have landed; the rest are honestly deferred with their rationale.

- [x] `anarchie fsck` - full integrity check of the store against the RM. *Walks every EHR's head Compositions, parses each as canonical JSON, and validates it against the RM (and its claimed template, if registered), reporting anything that fails to parse or conform and exiting non-zero. Because the files are the system of record, integrity is verifiable at any time independent of the index.*
- [x] **Archetype packs** - `anarchie pack add <name>` for installable OPT sets (see [bundled-archetypes.md](bundled-archetypes.md)). *MVP: `pack add` installs the bundled `ips-core` pack (the starter set) or every `*.opt.json` in a local directory, and `pack list` shows the bundled packs. The networked registry / `kam` ([Knowledge Artefacts Package Manager](https://github.com/pacharanero/knowledge-artefacts-package-manager)) integration is the next step.*
- [ ] **`sct` integration** - terminology binding validation via the `sct` binary / FHIR `$validate-code`. *Deferred (external): needs the `sct` binary and an operator-supplied SNOMED licence; the validator already isolates terminology to a future optional backend, so this is an additive seam, not a rework.*
- [ ] **`gitehr` convergence** - investigate one git repository carrying both a gitehr journal/state view and anarchie Compositions over a shared history. *Deferred (research/external).*
- [ ] **EEHRxF/FHIR projection** - project Compositions into FHIR resources for IPS / EHDS Patient Summary / xDHR implementation guides, as a derived consumer layer (see [regulatory-context.md](regulatory-context.md)). A convenience projection, *not* a certified EHDS gateway. *Deferred (a self-contained consumer-layer workstream).*
- [x] Basic install: `cargo install --git` plus an interim `curl | sh` one-liner (builds from source). See the [installation page](https://pacharanero.github.io/anarchie/install/).
- [ ] Full release pipeline: the *bump-on-`main`, CI-does-the-rest* model - cargo-dist prebuilt binaries, a shared Homebrew tap, and a Windows MSI on the first tagged release; then additive `.deb` / `.rpm` / `.dmg` jobs and a crates.io publish (`cargo install anarchie`). *Staged after a crates.io namespace is confirmed.*
- [ ] Optional `tui` / `gui` for browsing an EHR. *Deferred.*

**Learning milestone:** do the three projects (`sct`, `gitehr`, `anarchie`) actually compose into something greater than the sum? *Open. What this phase established concretely is that the bundling mechanism generalises cleanly to installable packs, and that the file-is-truth design makes a standalone `fsck` trivial and trustworthy. The cross-project composition (terminology via `sct`, history via `gitehr`) remains the genuinely speculative, still-open question.*

---

## Guiding constraints throughout

- **Always shippable** - every phase ends with a runnable binary and inspectable files.
- **Single binary, no runtime** - no JVM in the shipped artefact, ever. (Archie is a test-time oracle only.)
- **Files stay legible** - the on-disk format remains greppable and git-friendly; derived data stays segregated and disposable.
- **Honest scope** - features land behind the conformance/scaling limits stated in [scaling.md](scaling.md), not over-promised.
- **Clear licensing** - the four-layer split (code AGPL-3.0-or-later / specs CC-BY-ND / archetypes CC-BY-SA / terminology not bundled) is maintained at every release, per [licensing.md](licensing.md).
- **Learning over completeness** - this is an experiment; a working subset that teaches us something beats a broken attempt at full coverage.
