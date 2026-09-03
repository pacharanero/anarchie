<!--
SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Rust SDK

## Decision

`anarchie` consumes an existing, published Rust openEHR SDK rather than growing and extracting its own. The [FerroEHR](https://github.com/rubentalstra/FerroEHR) `openehr-*` crates are that SDK.

This supersedes the earlier plan to extract `rm`, `aom`, `opt`, and `validate` from this repository as `anarchie-*` leaf crates. That plan was correct when no suitable published SDK existed. One now does, generated from the openEHR BMM and released to crates.io, and it is more complete than anything this project would write by hand. Maintaining a second hand-written Reference Model would be duplicated effort producing a strictly worse artefact.

What does not change is the boundary itself. The host-independence rules below were written as constraints on crates this project would publish; they now serve as **admission criteria for a dependency**. A crate that fails them does not enter the kernel.

## What anarchie remains

Adopting the SDK narrows what this project claims to be, and sharpens it.

`anarchie` is a local-first, file-and-git openEHR CDR. Its original contributions are the immutable canonical-JSON store, git-native versioning, the CLI, the MCP server, the embedded SQLite query path, and the knowledge-package manager. None of those exist in FerroEHR, whose CDR decomposes canonical JSON into mutable PostgreSQL rows and requires a database server.

The two projects therefore share a model layer and diverge entirely at persistence. That is the collaboration: a common SDK, two honest experiments in what to do with it.

**The SDK crates impose no persistence choice.** They carry no database, network, filesystem, or async-runtime dependency in their runtime graphs; PostgreSQL is confined to FerroEHR's `app/ferroehr`. File-based persistence remains this project's whole point and is unaffected.

## Kernel admission criteria

A crate may sit below the CDR boundary only if it is a pure transformation over caller-provided values. It must not depend on the filesystem, git, a database, HTTP or MCP, a CLI, a clock, an async runtime, or process-global state. Feature flags that pull any of those must be switched off, and the resulting build verified, not assumed.

The CDR layers (`store`, `query`, `serve`, `cli`) are adapters above this line. They may depend on the kernel; the kernel may never depend on them.

## Adoption sequence

Each stage is independently shippable and independently revertable. Nothing later is a prerequisite for anything earlier being useful.

### Stage 1 - AQL parsing (`openehr-query`) - done

`src/query/aql/{lexer,parser}.rs` are replaced by `openehr-query` 0.0.56 plus `src/query/aql/lower.rs`. `src/query/execute.rs` and `src/query/index.rs` are untouched: FerroEHR's own executor compiles to PostgreSQL SQL and is not a crate, so the embedded SQLite path stays ours.

`ast.rs` was **kept**, against the original sketch. The SDK type is a syntax tree spanning all of AQL 1.1; `ast.rs` is the executor's input contract, covering only what the index can answer. Collapsing the two would have pushed grammar-shaped types through the query planner and made every unsupported construct an executor concern. Keeping them separate puts the whole narrowing in one reviewable file and leaves `parse()`'s signature unchanged, so the executor, stored queries, and the conformance corpus needed no edit.

The lowering is fail-closed in the same sense as the OPT importer: valid AQL that anarchie cannot execute is refused by naming the construct, never silently narrowed to something runnable. The conformance corpus is what makes that verifiable, and it caught nothing on the swap - the two queries anarchie deliberately refuses are still refused, now at lowering rather than at parse.

Measured cost: **14 new transitive crates and +1.2 MiB on the release binary (8.8 -> 10.0 MiB, +13%)**, against 646 lines of hand-written lexer and parser removed. It also sets the project's 1.96 MSRV.

### Stage 2 - Reference Model (`openehr-base`, `openehr-rm`, transitively `openehr-term`)

Replace `src/rm/` and the type-level parts of `src/validate/rm.rs`. This is the substantial win: a BMM-generated RM, terminology-backed class invariants, and an RM path engine, against roughly 520 lines of hand-written subset.

**This stage touches the byte-stability invariant and must be gated accordingly.** The SDK emits canonical JSON as `_type`-first in BMM declaration order; `anarchie` currently emits serde field order. Both are deterministic, so byte-stability survives the change, but the bytes differ. That makes it a one-time re-canonicalisation of every stored Composition, requiring a golden-vector diff and a documented store migration. Pretty-printing remains this project's own serializer concern and is orthogonal.

### Stage 3 - Templates and renderer formats (`openehr-its`)

Retire the hand-rolled legacy OPT XML importer in `src/opt.rs` in favour of the crate's `opt14` model and JSON codec, then take `flat` and `webtemplate` to deliver the **Renderer formats** and **Explorer interoperability** roadmap items without writing them.

Blocked on an upstream change: see [Upstream dependencies](#upstream-dependencies) below.

### Stage 4 - Archetype model (`openehr-am`, `openehr-adl`)

Only when ADL2/OPT2 becomes real work rather than a roadmap line.

### Not adopted

`app/ferroehr` (the PostgreSQL node model) and `app/ferroehr-rest` (bound to a concrete service type with no backend trait seam) are outside the boundary and stay outside it.

## Upstream dependencies

Two limitations block or complicate adoption. Both are ordinary upstream work, and the project's stated posture is to contribute the fix rather than fork or work around it.

- **`openehr-its` has no granular features.** It exposes only `default = ["full"]`, and `full` pulls `axum`, `http`, `moka`, and `jsonschema`. Taking it with `default-features = false` leaves effectively nothing. A non-server consumer that wants the canonical-JSON codec, the `opt14` model, or the FLAT/WebTemplate machinery cannot currently avoid taking an HTTP framework and an async cache. **Stage 3 is blocked on separable features** (for example `json`, `xml`, `opt14`, `flat`, `rest`) and that is the first issue to raise.
- **The OPT constraint validator is not a crate.** It lives in `app/ferroehr/src/validation/opt/`, roughly 2,500 lines, coupled to the application only through its error type. `anarchie`'s `src/validate/opt.rs` is exactly the second consumer that would justify lifting it into the crate set, and offering to do that work is a well-scoped opening contribution. Until then this project keeps its own template validator.

## Risks accepted

Recorded plainly because they are the reason this is a decision rather than a default.

- **Bus factor of one.** FerroEHR has a single maintainer, no organisation, and no legal entity behind it; the project states this itself in `MAINTAINERS.md` and treats it as a finding rather than a footnote. It is a serious dependency risk on a project that is nonetheless unusually rigorous about disclosing it.
- **`0.0.x` versioning.** No semver stability promise, and the published crates already run ahead of the in-repo workspace version. Pin exactly and expect churn.
- **MSRV rose to 1.96** at Stage 1, set by the crates themselves.
- **Dependency footprint grows** (`rust_decimal`, `chumsky`, `logos`, `indexmap`, `roxmltree`, `serde_jcs`, `serde_path_to_error`, `stacker`, and more) against a project whose distinguishing claim is a light single binary with no runtime. Stage 1 alone cost 14 crates and +13% binary size. Measure before and after each stage; the single-binary promise is about not shipping a JVM or a database server, not about a small dependency tree, but the trade should be observed rather than ignored.

The mitigation for all four is the same and is cheap: the crates are MIT and Apache-2.0, so a fork is always available and never needs permission.

## Licensing

The `openehr-*` crates are `MIT AND Apache-2.0`. `openehr-term` adds `CC-BY-SA-3.0` for the openEHR TERM 3.1.0 XML bundle it embeds. Permissive dependencies compose into an AGPL-3.0-or-later work one way, so adoption is compatible and `anarchie`'s own code licence is unchanged.

The CC-BY-SA-3.0 terminology bundle is a redistribution obligation, and it sits naturally in the existing four-layer split alongside the CKM archetype derivatives already carried at 3.0. Record it in [licensing.md](licensing.md) and keep `reuse lint` green when Stage 2 lands.

## GitEHR

The earlier plan made GitEHR the trigger for extracting an `anarchie` SDK. It is now simpler: GitEHR consumes the same `openehr-*` crates directly. Nothing has to be extracted, published, or version-managed by this project first, and the two products share a model layer without either depending on the other.

The shared conformance obligation survives the change. The first GitEHR integration should still add positive and negative cases proving that the same canonical Composition parses, serialises byte-stably where the profile guarantees it, and validates equivalently in both products.

## Reversal

If FerroEHR becomes unmaintained, diverges from the specification, or takes a direction this project cannot follow, the exit is to fork the crates under their existing permissive licences and continue. That is materially cheaper than the alternative this decision replaces, which was to write and maintain the same model layer from scratch indefinitely.
