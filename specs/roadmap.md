# Roadmap

`anarchie` is primarily a **learning and experimentation project**, so it optimises for *learning something at each step* and for *always having a working artefact*, rather than racing to feature-completeness. Nothing here depends on a later item to be useful.

Status legend: `[~]` in progress / partial · `[ ]` not started. Completed work is removed from this file; the shipped command surface is summarised in [docs/reference/roadmap.md](../docs/reference/roadmap.md) and documented per command on the docs site. (The *Guiding constraints* at the foot are standing principles, not tasks, so they carry no box.)

---

## Open and deferred work

### Conformance and correctness

The biggest open question is not "does it run" but "is it *correct* against the reference implementations". Both cross-checks are **test-time oracles only** - never a runtime dependency, so the single-binary promise holds.

- [~] **Validator vs Archie.** `s/archie-conformance` now runs six shared valid/invalid Compositions through the native validator and a source-pinned Archie 3.17.0 ADL2/OPT2 oracle. The first differential exposed and fixed a skipped `C_CODE_PHRASE` constraint on monomorphic `DV_CODED_TEXT`. Remaining work is nested archetype repositories, wider RM/data-value and OPT constraints, and a larger externally sourced corpus; the published [openEHR/specifications-CNF](https://github.com/openEHR/specifications-CNF) repository provides conformance schedules but not a ready-made validator-vector corpus. See [validation.md](validation.md).
- [~] **REST/AQL vs the EHRbase sandbox.** Submit the same template, Composition, and AQL to both and compare responses, to quantify "mostly works" for the server layer. `s/ehrbase test` now runs 23 differential AQL cases (COUNT, projection, all comparison operators, boolean logic, MATCHES, ORDER BY, LIMIT/OFFSET, all five aggregates) against a pinned EHRbase 2.34.0 oracle; remaining gap is REST endpoint comparison, multi-composition datasets, and known EHRbase limitations (LIKE on leaf paths, EXISTS, composition-name/ehr-id as queryable paths).

### openEHR SDK adoption

Adopt the published [FerroEHR](https://github.com/rubentalstra/FerroEHR) `openehr-*` crates (crates.io, `MIT AND Apache-2.0`) as the model layer instead of hand-writing and extracting our own. They are generated from the openEHR BMM and carry no database, network, filesystem, or async-runtime dependency, so file-based persistence is unaffected. The rationale, admission criteria, risks, and reversal path are in [rust-sdk.md](rust-sdk.md). Each stage is independently shippable. Stage 1 has landed: AQL is now parsed by `openehr-query` and narrowed to the executable subset by `src/query/aql/lower.rs`, at a cost of 14 transitive crates, +13% binary size, and a 1.96 MSRV. That measurement is the yardstick for the stages below.

- [ ] **Stage 2 - `openehr-base` + `openehr-rm` (+ `openehr-term`).** Replace `src/rm/` and the type-level parts of `src/validate/rm.rs` with the BMM-generated RM, terminology-backed invariants, and RM path engine. Gated: the SDK emits `_type`-first BMM declaration order where we emit serde field order, so this is a one-time re-canonicalisation of the store needing a golden-vector diff and a documented migration. Also record the crate's embedded CC-BY-SA-3.0 TERM bundle in [licensing.md](licensing.md).
- [ ] **Stage 3 - `openehr-its`.** Retire the hand-rolled OPT XML importer in `src/opt.rs` for the crate's `opt14` model, then take `flat`/`webtemplate` to deliver the **Renderer formats** and **Explorer interoperability** items below. **Blocked upstream:** the crate exposes only `default = ["full"]`, which pulls `axum`, `http`, `moka`, and `jsonschema` - a breach of the kernel rule. Asked for separable features (`json`, `xml`, `opt14`, `flat`, `rest`) on [FerroEHR#2807](https://github.com/rubentalstra/FerroEHR/issues/2807), offering the PR once the maintainer names the grouping.
- [ ] **Stage 4 - `openehr-am` / `openehr-adl`.** Only when ADL2/OPT2 becomes real work.
- [ ] **Upstream the OPT constraint validator.** FerroEHR's template validator sits in `app/ferroehr/src/validation/opt/` (~2,500 lines) rather than a crate, coupled to the app only by its error type. Our `src/validate/opt.rs` is the second consumer that justifies lifting it; offer to do the work. Keep our own validator until then.

### Templates and serialisation

- [~] **Batteries-included knowledge packages.** The seed exists as eight embedded starter templates, default installation, `--minimal`, local-directory packs, attribution, template validation, deterministic CKM source inventory, deployment-aware source-policy resolution into `knowledge.lock`, reproducible verified package archives installed and audited in content-addressed deployment storage, and a closed-graph CKM source-package publisher with provenance evidence. The target is the complete eligible published International CKM archetype library plus a conformance-tested executable template catalogue; embedding, activation/removal/rollback policy, compiled templates, version-safe runtime integration, package registries and updates, and release gates remain. See [knowledge-packages.md](knowledge-packages.md).
- [~] **Ingest `.opt` XML** exported from Archetype Designer / the ADL Workbench into the AOM tree, alongside anarchie's native flattened-JSON OPT form ([serialisation-formats.md](serialisation-formats.md)). `anarchie template add` now lowers a safe legacy ADL 1.4 subset (complex/archetype-root objects, attributes, inclusive intervals, `C_DV_QUANTITY` units/magnitude/precision, and `C_CODE_PHRASE`) into native JSON and rejects unsupported constraints rather than silently weakening validation. Quantity property constraints, slots/rules/defaults, ordered/unique cardinality, real externally sourced template corpus coverage, the ADL2/OPT2 policy, and re-generating bundled templates remain.
- [ ] **Renderer formats** - Web Template generation on template registration, and FLAT / STRUCTURED conversion at the REST boundary, which a form renderer expects. A self-contained serialisation workstream; the store and wire format stay canonical JSON. Expected to be delivered by SDK adoption Stage 3 rather than written here.
- [ ] **Desktop-client interoperability (near term).** Treat [openEHR Explorer](https://github.com/platzhersh/openehr-explorer) and [ehr-ctrl](https://www.ehr-ctrl.info/) as independent real-client compatibility targets, not dependencies. Exercise the same black-box smoke-test matrix against `anarchie serve`: connection setup, EHR create/read, canonical-JSON Composition create/read, template list/upload, ad hoc AQL, and stored-query lifecycle where each client exposes the operation. Capture request/response fixtures and document unsupported or vendor-specific behaviour separately for each client. Then derive Web Templates and support the read paths openEHR Explorer needs to browse, inspect, and query an anarchie deployment; finally add FLAT writes and a form-submission round trip. Assess a `/rest/openehr/v1` compatibility prefix only against that evidence. The Web Template and FLAT machinery this needs comes from SDK adoption Stage 3. Keep FLAT, STRUCTURED, and Web Templates derived at the service boundary, and keep desktop UI frameworks, credentials, and vendor-specific adapters outside anarchie.
- [ ] **OMOP derived view and mappings.** Export canonical, versioned Composition data into an OMOP Common Data Model-compatible analytical view, with explicit terminology mappings and source provenance back to each Composition version. OMOP is an analytics and observational-research projection, never a replacement for the openEHR clinical system of record.

### Query

- [ ] **DuckDB/Parquet analytics path** alongside the SQLite path-index, for column-oriented aggregates. The SQLite path already covers the MVP aggregates (`COUNT`/`MIN`/`MAX`/`SUM`/`AVG`); the open question is how much of AQL a DuckDB-over-JSON approach handles before a bespoke engine is unavoidable. Additive, isolated to the analytics tier ([query-engine.md](query-engine.md)).

### Integration and convergence (speculative)

Integrations with external systems and open research questions, deliberately left open.

- [ ] **`sct` terminology binding** - validate terminology bindings via the `sct` binary / FHIR `$validate-code`. The validator already isolates terminology to an optional backend, so this is an additive seam, not a rework ([validation.md](validation.md)).
- [ ] **`gitehr` convergence** - one git repository carrying both a gitehr journal/state view and anarchie Compositions over a shared history.
- [ ] **EEHRxF / FHIR projection** - project Compositions into FHIR resources for IPS / EHDS Patient Summary / xDHR, as a derived consumer layer. A convenience projection, **not** a certified EHDS gateway. The gap analysis and plan are in [ips-readiness.md](ips-readiness.md); the regulatory framing is in [regulatory-context.md](regulatory-context.md).

### Distribution

`cargo install` and an interim `curl | sh` one-liner build from source today (see the [installation page](https://pacharanero.github.io/anarchie/install/)).

- [~] **Full release pipeline** - the *bump, CI-does-the-rest* model is now wired: `s/version++` bumps + changelogs + tags, and the pushed tag drives both the crates.io publish and the cargo-dist release (prebuilt binaries for five targets, a Homebrew formula, and a Windows MSI). Validated by `dist plan`; proven end to end only on the first real tag. Still open: the additive `.deb` / `.rpm` / `.dmg` targets, and the `HOMEBREW_TAP_TOKEN` secret + `pacharanero/homebrew-tap` repo the formula-push needs.

### Community and polish

- [ ] **Circulate for critique** - the openEHR Discourse, and the overlap with `gitehr`.
- [ ] **Verify the CKM Terms of Use** wording and quote it in the bundle attribution before a release is cut (a packaging-time step).
- [ ] Optional **`tui` / `gui`** for browsing an EHR.

---

## House-style conformance

Conformance with the shared engineering standards in `~/code/house-style` - the conventions every Baw Medical repo (`sct`, `dsc`, `gitehr`) follows. Only remaining or partial work is listed here. See also [HOUSE-STYLE-AUDIT.md](../HOUSE-STYLE-AUDIT.md) for the point-in-time audit these items trace to.

### CLI shape

- [ ] *(Lower priority)* a machine-discoverable `--schema` / fillable-template surface, sharing one schema with the MCP tools.

### Release cascade and distribution

Extends the **Distribution** item above with the remaining house-style check:

- [~] `cargo binstall anarchie` works off cargo-dist's release manifest (no explicit `[package.metadata.binstall]` needed, matching `dsc`); confirm on the first release.

### Testing hygiene

- [~] The Archie and EHRbase cross-checks now provide initial golden-vector layers (tracked under **Conformance** above); wider externally sourced validation and REST corpora, machine-readable profiles, renderer/package cases, and release evidence reports remain open. See [conformance.md](conformance.md).

### Reusable Rust SDK

Superseded: `anarchie` now consumes the published FerroEHR `openehr-*` crates as its SDK rather than extracting its own. The extraction plan is withdrawn; the host-independence rules survive as admission criteria for a dependency. See [rust-sdk.md](rust-sdk.md).

- [~] **Kernel boundary.** `rm`, `aom`, `opt`, and `validate` remain the host-independent kernel below the CDR adapters. Keep it free of filesystem, git, SQLite, HTTP/MCP, CLI, clock, async-runtime, and global-state dependencies, whether the code is ours or a crate.

---

## Guiding constraints throughout

- **Always shippable** - every step ends with a runnable binary and inspectable files.
- **Single binary, no runtime** - no JVM in the shipped artefact, ever. (Archie and EHRbase are test-time oracles only.)
- **Files stay legible** - the on-disk format remains greppable and git-friendly; derived data stays segregated and disposable.
- **Honest scope** - features land behind the conformance/scaling limits stated in [scaling.md](scaling.md), not over-promised.
- **Clear licensing** - the four-layer split (code AGPL-3.0-or-later / specs CC-BY-ND / archetypes CC-BY-SA / terminology not bundled) is maintained at every release, per [licensing.md](licensing.md).
- **Learning over completeness** - this is an experiment; a working subset that teaches us something beats a broken attempt at full coverage.
