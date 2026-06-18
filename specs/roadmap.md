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

- [ ] Curate the Tier 1 template set from openEHR International **Published** archetypes (see [bundled-archetypes.md](bundled-archetypes.md)).
- [ ] Build the OPTs (build-time flatten via Archetype Designer / ADL Workbench / Archie).
- [ ] Dual-license: code **AGPL-3.0-or-later**, bundled OPTs under **CC-BY-SA 3.0** with a provenance/`NOTICE` manifest (see [licensing.md](licensing.md)).
- [ ] `anarchie init --with-starter-templates` (default) / `--minimal`.
- [ ] Verify CKM Terms of Use wording and quote it in the bundle NOTICE before shipping.

**Learning milestone:** can a newcomer store a believable patient record (vitals, problems, meds, allergies, results) within five minutes of install?

---

## Phase 4 — Query (AQL)

**Goal:** answer population queries, not just id lookups.

- [ ] `anarchie index` - flatten Compositions into a path-value table in SQLite.
- [ ] Index freshness tracking in the manifest; `--rebuild` backstop.
- [ ] AQL parser (Rust grammar) for the MVP subset.
- [ ] AQL → SQL translation over the path index.
- [ ] `anarchie aql "SELECT …"` returning an openEHR ResultSet.
- [ ] Stored (named) queries registered as git-versioned data, executed by name and version.
- [ ] DuckDB/Parquet path for aggregate analytics (explore how much it gives for free).

**Learning milestone:** how much of AQL can DuckDB-over-JSON handle before a bespoke engine is unavoidable?

---

## Phase 5 — Services (REST + MCP)

**Goal:** interoperate with the existing openEHR ecosystem and with AI agents.

- [ ] `anarchie serve` - openEHR REST API (Phase 1 surface from [rest-api.md](rest-api.md)): EHR + Composition CRUD, `If-Match` concurrency.
- [ ] AQL endpoint (ad-hoc + stored queries).
- [ ] Template definition endpoints, plus example-Composition generation from a template.
- [ ] Web Template generation on template registration; FLAT / STRUCTURED conversion at the REST boundary (see [serialisation-formats.md](serialisation-formats.md)).
- [ ] Cross-check REST/AQL behaviour against the EHRbase sandbox as a test-time oracle.
- [ ] `anarchie mcp` - stdio MCP server: get/commit/validate/query Compositions for LLM agents, reusing the structured violation output to let an agent self-correct.

**Learning milestone:** can an existing openEHR app or form renderer point at `anarchie serve` and work?

---

## Phase 6 — Integration and convergence (speculative)

**Goal:** explore the interesting collisions.

- [ ] **`sct` integration** - terminology binding validation via the `sct` binary / FHIR `$validate-code`.
- [ ] **`gitehr` convergence** - investigate one git repository carrying both a gitehr journal/state view and anarchie Compositions over a shared history.
- [ ] **Archetype packs** - `anarchie pack add <name>` for installable OPT sets, ideally consuming `kam` ([Knowledge Artefacts Package Manager](https://github.com/pacharanero/knowledge-artefacts-package-manager)) packages rather than reinventing packaging (see [bundled-archetypes.md](bundled-archetypes.md)).
- [ ] **EEHRxF/FHIR projection** - project Compositions into FHIR resources for IPS / EHDS Patient Summary / xDHR implementation guides, as a derived consumer layer (see [regulatory-context.md](regulatory-context.md)). A convenience projection, *not* a certified EHDS gateway.
- [ ] `anarchie fsck` - full integrity check of the store against the RM.
- [ ] Distribution: prebuilt binaries, `curl | sh` installer, Homebrew/Scoop - mirroring `sct`'s release pipeline.
- [ ] Optional `tui` / `gui` for browsing an EHR.

**Learning milestone:** do the three projects (`sct`, `gitehr`, `anarchie`) actually compose into something greater than the sum?

---

## Guiding constraints throughout

- **Always shippable** - every phase ends with a runnable binary and inspectable files.
- **Single binary, no runtime** - no JVM in the shipped artefact, ever. (Archie is a test-time oracle only.)
- **Files stay legible** - the on-disk format remains greppable and git-friendly; derived data stays segregated and disposable.
- **Honest scope** - features land behind the conformance/scaling limits stated in [scaling.md](scaling.md), not over-promised.
- **Clear licensing** - the four-layer split (code AGPL-3.0-or-later / specs CC-BY-ND / archetypes CC-BY-SA / terminology not bundled) is maintained at every release, per [licensing.md](licensing.md).
- **Learning over completeness** - this is an experiment; a working subset that teaches us something beats a broken attempt at full coverage.
