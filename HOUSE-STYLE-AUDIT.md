<!--
SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# House-Style Audit

Audit date: 2026-07-06

Audited against: `~/code/house-style` as of this date, especially `agents.md`, `new-repos.md`, `security.md`, `ci.md`, `distribution.md`, `commits.md`, `rust-cli.md`, `licensing.md`, `scripts.md`, `docs.md`, `testing.md`, `specs.md`, and `clinical-safety.md`.

Scope: lightweight static audit only. No files were changed except this report.

## Summary

`anarchie` is a single-crate Rust CLI + library: an experimental, offline-first openEHR Clinical Data Repository, with a Zensical docs site and a substantial `specs/` design set. It is public (`github.com/pacharanero/anarchie`) and explicitly not for clinical use.

Overall alignment is **high**. A concentrated push over the last few days closed most of the house-style gaps: REUSE 3.3 licensing, a hardened `ci.yml`, Dependabot, the full `s/` script set, artifact-based Pages deploy, the CLI overhaul (`--format`, `version`, completions, bare-help, SIGPIPE reset, thin `main` + `cli/` modules), and conventional commits throughout. What remains is a small number of genuine gaps rather than a systemic mismatch.

Main improvements:

- Add a `permissions:` block to `publish-crates.yml` - the one workflow that carries the crates.io token currently runs with the default (broad) `GITHUB_TOKEN` scope.
- Add an agent entry point (`agent-instructions.md` + thin `AGENTS.md` / `CLAUDE.md`); the repo has none despite being heavily agent-developed.
- Add a public-repo `SECURITY.md` vulnerability-reporting policy.
- Finish the release cascade (`s/version++`, `cliff.toml`, `CHANGELOG.md`, cargo-dist) - already tracked in `specs/roadmap.md`.

## Priority Findings

### P1 - `publish-crates.yml` runs without a least-privilege `permissions:` block

Evidence:

- `.github/workflows/publish-crates.yml` has `on: push: tags: ["v*"]` and `workflow_call:`, holds `secrets.CARGO_REGISTRY_TOKEN`, but declares no `permissions:` at workflow or job level, so it inherits the repository-default `GITHUB_TOKEN` scope.
- By contrast `.github/workflows/ci.yml` (line 13) and `.github/workflows/docs.yml` (line 37) both set explicit `permissions:`.

House style:

- `ci.md` requires least-privilege `GITHUB_TOKEN` (`permissions: contents: read`), and `security.md` treats supply-chain hardening as expected. The publish workflow is the most sensitive one in the repo, so it is the most important place to scope the token down.

Suggested change:

- Add `permissions: contents: read` to `publish-crates.yml` (publishing to crates.io uses `CARGO_REGISTRY_TOKEN`, not `GITHUB_TOKEN`, so no write scope is needed).
- While there: switch the raw `actions/cache` step to `Swatinem/rust-cache` to match `ci.yml`, and gate the publish with `cargo build --release --locked` / `cargo test --locked` for a reproducible release build.

### P2 - No agent instruction file

Evidence:

- No `AGENTS.md`, `agent-instructions.md`, or `CLAUDE.md` at the repo root (confirmed by directory listing). Design intent instead lives across `README.md` and 15 files under `specs/`.

House style:

- `agents.md` and `new-repos.md` both list an agent entry point among the minimum files. Its purpose is not to restate the README but to give an agent a read-first order, the core invariants, and copy-pasteable validation commands.

Suggested change:

- Add a vendor-neutral `agent-instructions.md` using the template in `agents.md`, with thin `AGENTS.md` and `CLAUDE.md` pointers. It should capture the invariants that are already true in this codebase but currently undocumented for agents, for example: the canonical JSON file is the source of truth and the index is a disposable derived view; validation runs at the door (nothing non-conformant reaches git); the shipped artefact is a single binary with no runtime beyond `git` (Archie/EHRbase are test-time oracles only); the four-layer licensing split is maintained. Point "Before every commit" at `s/lint` + `s/test` and "Read first" at the README, `specs/roadmap.md`, and `~/code/house-style/AGENTS.md`.

### P2 - No `SECURITY.md`

Evidence:

- No `SECURITY.md` tracked in the repo; it is a public repo (docs published at `pacharanero.github.io/anarchie`).

House style:

- `new-repos.md` lists `SECURITY.md` as a minimum file, and `security.md` says public repos should have a vulnerability-reporting policy.

Suggested change:

- Add a short `SECURITY.md` with a private reporting channel (email or GitHub private vulnerability reporting) and a note that anarchie is experimental and not for clinical use, so found issues are not patient-safety incidents in a deployed system.

### P2 - Release cascade is incomplete

Evidence:

- No `s/version++`, `cliff.toml`, `CHANGELOG.md`, or cargo-dist `release.yml`; `Cargo.toml` has no `[package.metadata.dist]` or `[package.metadata.binstall]`. `publish-crates.yml` covers crates.io only.
- Already tracked as `[ ]` items under "Release cascade and distribution" in `specs/roadmap.md`.

House style:

- `distribution.md` and `commits.md` define releasing as one action - `s/version++ [patch|minor|major]` bumps the version, regenerates a git-cliff `CHANGELOG.md`, and lands `chore(release): vX.Y.Z` - with cargo-dist producing prebuilt binaries, a Homebrew tap, and an MSI.

Suggested change:

- Add `cliff.toml` + an initial `CHANGELOG.md` and an `s/version++` script (git-cliff, `cargo-set-version`, and `dist` are already installed locally), then a cargo-dist `release.yml`. Adapt `~/code/discourse/dsc` (its `cliff.toml`, `release.yml`, and `[workspace.metadata.dist]` are the closest living example). Homebrew publishing needs a `HOMEBREW_TAP_TOKEN` secret and a `pacharanero/homebrew-tap` repo; flag both as prerequisites.

### P3 - Missing `.editorconfig`

Evidence:

- No `.editorconfig` at the repo root.

House style:

- `new-repos.md` lists `.editorconfig` (indentation, line endings, final newline) among the minimum files.

Suggested change:

- Add a small `.editorconfig` (UTF-8, LF, final newline, 4-space Rust / 2-space YAML-JSON-Markdown) consistent with what `cargo fmt` already enforces.

### P3 - No `CONTRIBUTING.md` for a public repo

Evidence:

- No `CONTRIBUTING.md`; the README has a licensing section and setup, but no contribution guide.

House style:

- `new-repos.md` says add `CONTRIBUTING.md` when the project is collaborative or public. This is a public repo circulated (per the roadmap) for openEHR-community critique.

Suggested change:

- Add a short `CONTRIBUTING.md` pointing at `s/test` / `s/lint`, the conventional-commit convention, and the opt-in `s/install-hooks`. Can be deferred until external contribution is actually invited.

### P3 - No `cargo audit` / advisory check in CI

Evidence:

- `.github/workflows/ci.yml` runs fmt, clippy, test, REUSE, and coverage, but no dependency-advisory scan.

House style:

- `security.md` and `ci.md` call for `cargo audit` (or an equivalent advisory check) alongside pinned actions and Dependabot.

Suggested change:

- Add a `cargo audit` job (e.g. via `taiki-e/install-action`, which the coverage job already uses) or `rustsec/audit-check`. Low urgency given the dependency-light tree.

### P3 - Consider `specs/ubiquitous-language.md`

Evidence:

- `specs/` has 15 design docs but no dedicated glossary; openEHR terminology (Composition, CONTRIBUTION, `version_uid`, archetype, OPT, AQL, EHR_STATUS) is used heavily across README, docs, CLI, and code.

House style:

- `specs.md` recommends `ubiquitous-language.md` where terminology matters, to keep canonical terms consistent across CLI commands, docs, code identifiers, and tests.

Suggested change:

- Optional: distil a short canonical-terms glossary. Much of the raw material already exists in `specs/reference-model-coverage.md` and `specs/openehr-terminology-codes.md`.

## Compliant / Good Patterns

- **Licensing / REUSE**: root `LICENSE`, `LICENSES/`, `REUSE.toml`, and SPDX headers; `reuse lint` reports full 3.3 compliance. The deliberate four-layer split (code AGPL-3.0-or-later / prose CC-BY-SA-4.0 / CKM archetypes CC-BY-SA-3.0 / openEHR specs not redistributed) is documented in `specs/licensing.md`. Matches `licensing.md`.
- **CI hardening**: `ci.yml` sets `permissions:`, has `workflow_dispatch`, pins every action to a full SHA with a `# vX.Y.Z` comment, runs a REUSE job, uses `Swatinem/rust-cache`, and adds a coverage job. Matches `ci.md`.
- **Dependency automation**: `.github/dependabot.yml` present (cargo + github-actions). Matches `ci.md`.
- **Docs**: Zensical site with artifact-based `upload-pages-artifact` + `deploy-pages` deploy, path filters, and `workflow_dispatch` (`docs.yml`). Matches `docs.md`.
- **Scripts**: full `s/` set (`build`, `test`, `lint`, `docs`, `install`, `install-hooks`) plus `s/README.md`; `s/docs` binds a free port dynamically; opt-in `.githooks/` + `s/install-hooks`. Matches `scripts.md`.
- **CLI shape**: global `--format text|json`, `version` subcommand, `clap_complete` completions, helpful bare invocation (exit 0), SIGPIPE reset, thin `main.rs` delegating to `cli/` modules with `clap` derive and `anyhow`. Matches `rust-cli.md` and the cross-cutting essentials.
- **Commits**: conventional throughout (`feat`, `fix`, `docs`, `ci`, `chore`, `refactor` with scopes). Matches `commits.md`.
- **Testing**: 69 tests including CLI end-to-end coverage; commit-signing disabled inside throwaway test git repos; a guard test keeps `examples/vitals.json` in step with the fixture. Matches `testing.md`.
- **README**: answers what / what-not / who / how / licence and states "not for clinical use" up front. Matches `new-repos.md`.
- **Specs**: durable design decisions in `specs/` with a checkbox-tracked `roadmap.md`. Matches `specs.md`.
- **crates.io publish**: wired and gated by build + test, triggered on `v*` tags.

## Not Applicable

- **`tauri-gui.md`**: no desktop GUI. An optional `tui` / `gui` is a deliberately deferred roadmap item, not a gap.
- **`presentations.md`**: no slide decks in this repo.
- **`skills.md`**: not a skill-authoring repo.
- **`clinical-safety.md`**: not yet applicable - anarchie is explicitly experimental and not for clinical use, so no root `SAFETY.md` is required today. Trigger condition: if it is ever positioned to affect real care, add a Tier 1 `SAFETY.md` per the standard. Worth a one-line forward note but not a current finding.
- **`library-extraction.md`**: applicable in principle (the `rm` module could become a leaf crate) but deliberately deferred until an external consumer appears - already documented as such in `specs/roadmap.md`.

## Suggested First PR

Security and scaffolding quick wins (small, high-value, no behaviour change):

1. Add `permissions: contents: read` to `publish-crates.yml` (P1); optionally switch its cache to `Swatinem/rust-cache` and add `--locked`.
2. Add `agent-instructions.md` + thin `AGENTS.md` / `CLAUDE.md` pointers (P2).
3. Add `SECURITY.md` (P2).
4. Add `.editorconfig` (P3).

## Suggested Second PR

The release cascade (larger, already roadmapped):

1. Add `cliff.toml` and an initial `CHANGELOG.md`.
2. Add `s/version++ [patch|minor|major]` (bump, regenerate changelog, commit `chore(release): vX.Y.Z`).
3. Add cargo-dist `[package.metadata.dist]` + `release.yml` and `[package.metadata.binstall]`, adapting `~/code/discourse/dsc`; flag the `HOMEBREW_TAP_TOKEN` / `pacharanero/homebrew-tap` prerequisites.
