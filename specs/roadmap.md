# Roadmap

`anarchie` is primarily a **learning and experimentation project**, so it optimises for *learning something at each step* and for *always having a working artefact*, rather than racing to feature-completeness. Nothing here depends on a later item to be useful.

## Where things stand

The core CDR is built and runnable end to end:

- the **Reference Model** in Rust with byte-stable canonical-JSON round-trip ([reference-model-coverage.md](reference-model-coverage.md));
- the **git-backed file store** - one repository per EHR, `CONTRIBUTION`-as-commit, version history, diff ([versioning-and-git.md](versioning-and-git.md), [on-disk-format.md](on-disk-format.md));
- native **RM + Operational Template validation**, wired into commit so non-conformant data never reaches git ([validation.md](validation.md));
- an **IPS-aligned starter template set** installed by default ([bundled-archetypes.md](bundled-archetypes.md), [ips-readiness.md](ips-readiness.md));
- the **AQL query engine** over a derived path-index ([query-engine.md](query-engine.md));
- the **openEHR REST API** and a **stdio MCP server** ([rest-api.md](rest-api.md));
- store **`fsck`** and installable **archetype packs**.

The shipped command surface is summarised, reader-friendly, in [docs/reference/roadmap.md](../docs/reference/roadmap.md) and documented per command on the docs site. This file now tracks **what remains**.

---

## Open and deferred work

### Conformance and correctness

The biggest open question is not "does it run" but "is it *correct* against the reference implementations". Both cross-checks are **test-time oracles only** - never a runtime dependency, so the single-binary promise holds.

- **Validator vs Archie.** Run the same Compositions through Archie (JVM) and `anarchie` and assert the verdicts agree. Needs the JVM toolchain and a curated conformance corpus ([openEHR/specifications-CNF](https://github.com/openEHR/specifications-CNF)); independent of the validator's own design, and the project's biggest correctness question. See [validation.md](validation.md).
- **REST/AQL vs the EHRbase sandbox.** Submit the same template, Composition, and AQL to both and compare responses, to quantify "mostly works" for the server layer.

### Templates and serialisation

- **Ingest `.opt` XML** exported from Archetype Designer / the ADL Workbench into the AOM tree, alongside anarchie's native flattened-JSON OPT form ([serialisation-formats.md](serialisation-formats.md)). This is also the durable path for re-generating the bundled templates instead of hand-authoring against at-codes.
- **Renderer formats** - Web Template generation on template registration, and FLAT / STRUCTURED conversion at the REST boundary, which a form renderer expects. A self-contained serialisation workstream; the store and wire format stay canonical JSON.

### Query

- **DuckDB/Parquet analytics path** alongside the SQLite path-index, for column-oriented aggregates. The SQLite path already covers the MVP aggregates (`COUNT`/`MIN`/`MAX`/`SUM`/`AVG`); the open question is how much of AQL a DuckDB-over-JSON approach handles before a bespoke engine is unavoidable. Additive, isolated to the analytics tier ([query-engine.md](query-engine.md)).

### Integration and convergence (speculative)

Integrations with external systems and open research questions, deliberately left open.

- **`sct` terminology binding** - validate terminology bindings via the `sct` binary / FHIR `$validate-code`. The validator already isolates terminology to an optional backend, so this is an additive seam, not a rework ([validation.md](validation.md)).
- **`gitehr` convergence** - one git repository carrying both a gitehr journal/state view and anarchie Compositions over a shared history.
- **EEHRxF / FHIR projection** - project Compositions into FHIR resources for IPS / EHDS Patient Summary / xDHR, as a derived consumer layer. A convenience projection, **not** a certified EHDS gateway. The gap analysis and plan are in [ips-readiness.md](ips-readiness.md); the regulatory framing is in [regulatory-context.md](regulatory-context.md).

### Distribution

`cargo install` and an interim `curl | sh` one-liner build from source today (see the [installation page](https://pacharanero.github.io/anarchie/install/)).

- **Full release pipeline** - the *bump-on-`main`, CI-does-the-rest* model: cargo-dist prebuilt binaries, a shared Homebrew tap, and a Windows MSI on the first tagged release; then additive `.deb` / `.rpm` / `.dmg`. The **crates.io publish** workflow is already wired up and fires on the first `v*` tag, so `cargo install anarchie` works once published.

### Community and polish

- **Circulate for critique** - the openEHR Discourse, and the overlap with `gitehr`.
- **Verify the CKM Terms of Use** wording and quote it in the bundle attribution before a release is cut (a packaging-time step).
- Optional **`tui` / `gui`** for browsing an EHR.

---

## Guiding constraints throughout

- **Always shippable** - every step ends with a runnable binary and inspectable files.
- **Single binary, no runtime** - no JVM in the shipped artefact, ever. (Archie and EHRbase are test-time oracles only.)
- **Files stay legible** - the on-disk format remains greppable and git-friendly; derived data stays segregated and disposable.
- **Honest scope** - features land behind the conformance/scaling limits stated in [scaling.md](scaling.md), not over-promised.
- **Clear licensing** - the four-layer split (code AGPL-3.0-or-later / specs CC-BY-ND / archetypes CC-BY-SA / terminology not bundled) is maintained at every release, per [licensing.md](licensing.md).
- **Learning over completeness** - this is an experiment; a working subset that teaches us something beats a broken attempt at full coverage.
