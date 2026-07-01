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

## House-style conformance

Gaps against the shared engineering standards in `~/code/house-style` - the conventions every Baw Medical repo (`sct`, `dsc`, `gitehr`) follows. None of these change what anarchie *does*; they make it consistent with its siblings. Grouped by the standard they come from.

### Licensing and REUSE

- Add a root `LICENSE` file (the full AGPL-3.0-or-later text).
- Add the `SPDX-FileCopyrightText: <year> Dr Marcus Baw and Baw Medical Ltd` line above the existing `SPDX-License-Identifier` on every source file (only the identifier is present today).
- Add a root `REUSE.toml` covering files that cannot carry a header (Markdown, JSON, `Cargo.lock`, test fixtures), and enforce `reuse lint` in CI.
- Reconcile content licensing: house-style is CC-BY-SA-4.0 for written content, whereas anarchie's [licensing.md](licensing.md) uses a deliberate four-layer split (openEHR specs CC-BY-ND, CKM archetypes CC-BY-SA-3.0). Decide whether anarchie's *own* prose (docs, specs) adopts CC-BY-SA-4.0 while the openEHR-derived layers keep their upstream licences.

### CLI shape

- A global `--format text|json` flag honoured by every command, replacing the per-command `--json` on `validate`/`fsck`.
- A bare `anarchie` (or a bare command family like `anarchie ehr`) should print a helpful usage summary and exit 0, not error with exit 2.
- An `anarchie version` subcommand honouring `--format`, keeping `--version`/`-V` as the quick-check flags.
- Shell completions via `clap_complete`: `anarchie completions <shell>` / `install`, for bash/zsh/fish/powershell, with a generation test.
- Reset SIGPIPE on Unix so output pipes cleanly into `head`/`less`.
- Refactor the single large `main.rs` into one module per command family, each with a `run()`, dispatched from a thin `main` (the domain logic already lives in the library modules).
- *(Lower priority)* a machine-discoverable `--schema` / fillable-template surface, sharing one schema with the MCP tools.

### CI and automation

- Add `.github/dependabot.yml` (cargo + github-actions; weekly grouped minor/patch with a cooldown).
- Harden `ci.yml`: `permissions: contents: read`, a `workflow_dispatch` trigger, a REUSE-compliance job, and `Swatinem/rust-cache` in place of the raw `actions/cache`.
- *(Optional)* tracked `.githooks/` + `s/install-hooks`, with a pre-commit hook running `s/lint`.

### Release cascade and distribution

Extends the **Distribution** item above with the house-style specifics:

- `s/version++ [patch|minor|major]` as the single release action - bump the version, regenerate `Cargo.lock` and a git-cliff `CHANGELOG.md`, commit `chore(release): vX.Y.Z`, and push. Needs a `cliff.toml` and a `CHANGELOG.md`.
- cargo-dist `release.yml` for prebuilt binaries, the shared `pacharanero/homebrew-tap`, and a Windows MSI; then additive `.deb` / `.rpm` / `.dmg`, with one `SHA256SUMS` as the source of truth.
- `[package.metadata.binstall]` so `cargo binstall anarchie` finds the release archives.

### Scripts and docs

- Add the canonical `s/` scripts anarchie lacks - `s/test`, `s/build`, `s/lint` (and `s/version++` above); add `s/README.md` once there are more than three.
- Make `s/docs` bind the first free port in 8000-8030 (IPv6-aware) rather than a fixed port.
- Move docs deployment to the artifact-based `upload-pages-artifact` + `deploy-pages` method (no `gh-pages` branch), with path filters and a `workflow_dispatch` trigger.

### Testing hygiene

- Disable commit signing (`git config commit.gpgsign false`) inside the throwaway git repos the store/CLI tests create, so contributors with global commit signing do not hit spurious failures.
- The openEHR CNF conformance corpus and the Archie/EHRbase cross-checks (tracked under **Conformance** above) are the golden-vector layer the standard asks for.

### Library extraction

- If the openEHR Reference Model (`rm`, and perhaps `aom` / `validate`) earns external consumers, extract it as a leaf crate (serde-only, no host dependencies) via `git subtree split` so its history travels. Deliberately deferred - the single-crate simplification is the right default until a consumer appears.

---

## Guiding constraints throughout

- **Always shippable** - every step ends with a runnable binary and inspectable files.
- **Single binary, no runtime** - no JVM in the shipped artefact, ever. (Archie and EHRbase are test-time oracles only.)
- **Files stay legible** - the on-disk format remains greppable and git-friendly; derived data stays segregated and disposable.
- **Honest scope** - features land behind the conformance/scaling limits stated in [scaling.md](scaling.md), not over-promised.
- **Clear licensing** - the four-layer split (code AGPL-3.0-or-later / specs CC-BY-ND / archetypes CC-BY-SA / terminology not bundled) is maintained at every release, per [licensing.md](licensing.md).
- **Learning over completeness** - this is an experiment; a working subset that teaches us something beats a broken attempt at full coverage.
