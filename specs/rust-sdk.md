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

**This crate's licence is now durably safe, not just currently safe.** The maintainer confirmed the split is policy, not an artefact of one release: code generated from the openEHR specs stays Apache-2.0; hand-written code is BUSL-1.1 ([Discourse, 2026-09-24](https://discourse.openehr.org/t/ferroehr-a-new-rust-based-openehr-cdr-looking-for-testers/17230/39)). `openehr-base`/`-rm`/`-am`/`-lang` are entirely BMM-generated, so they sit on the permanently-open side of that rule by construction, not by the current state of one crate. That removes the "re-read the licence before this stage" caveat this file previously carried for Stage 2 specifically - it still applies to Stages 3 and 4, whose crates mix generated and hand-written code.

**This stage touches the byte-stability invariant and must be gated accordingly.** The SDK emits canonical JSON as `_type`-first in BMM declaration order; `anarchie` currently emits serde field order. Both are deterministic, so byte-stability survives the change, but the bytes differ. That makes it a one-time re-canonicalisation of every stored Composition, requiring a golden-vector diff and a documented store migration. Pretty-printing remains this project's own serializer concern and is orthogonal.

### Stage 3 - Templates and renderer formats (`openehr-its`)

Retire the hand-rolled legacy OPT XML importer in `src/opt.rs` in favour of the crate's `opt14` model and JSON codec, then take `flat` and `webtemplate` to deliver the **Renderer formats** and **Explorer interoperability** roadmap items without writing them.

**Blocked on licence, partially and temporarily.** `openehr-its` is `BUSL-1.1 AND Apache-2.0` because it mixes generated and hand-written code under one crate. The maintainer has committed to splitting it along the generated/hand-written line and confirmed he will file a tracking issue ([Discourse, 2026-09-24](https://discourse.openehr.org/t/ferroehr-a-new-rust-based-openehr-cdr-looking-for-testers/17230/39)). Once that lands: the canonical JSON/XML codec and the `opt14` model are generated from the ITS-XML/ITS-JSON schemas, so they should move to the new Apache-2.0 split - which is exactly what retiring the hand-rolled importer in `src/opt.rs` needs. `flat`/`webtemplate` and the hand-written ITS-REST server contract are not generated and will stay BUSL-1.1, so **Renderer formats and Explorer interoperability remain blocked** even after the split; only the OPT-ingestion half of Stage 3 is recoverable this way. Re-check the split's actual crate boundaries against this expectation when it ships - do not assume the grouping matches this paragraph until verified.

### Stage 4 - Archetype model (`openehr-am`, `openehr-adl`)

Only when ADL2/OPT2 becomes real work rather than a roadmap line. `openehr-am` is Apache-2.0 and available; `openehr-adl` is BUSL-1.1 and is not.

### Not adopted

`app/ferroehr` (the PostgreSQL node model) and `app/ferroehr-rest` (bound to a concrete service type with no backend trait seam) are outside the boundary and stay outside it.

## Upstream dependencies

Two limitations block or complicate adoption. Both are ordinary upstream work, and the project's stated posture is to contribute the fix rather than fork or work around it.

- **`openehr-its` has no granular features.** It exposes only `default = ["full"]`, and `full` pulls `axum`, `http`, `moka`, and `jsonschema`. Taking it with `default-features = false` leaves effectively nothing. A non-server consumer that wants the canonical-JSON codec, the `opt14` model, or the FLAT/WebTemplate machinery cannot currently avoid taking an HTTP framework and an async cache. **Stage 3 is blocked on separable features** (for example `json`, `xml`, `opt14`, `flat`, `rest`), raised on [FerroEHR#2807](https://github.com/rubentalstra/FerroEHR/issues/2807) with an offer to write the PR once the maintainer names the grouping.
- **The OPT constraint validator is not a crate.** It lives in `app/ferroehr/src/validation/opt/`, roughly 2,500 lines, coupled to the application only through its error type. `anarchie`'s `src/validate/opt.rs` is exactly the second consumer that would justify lifting it into the crate set, and offering to do that work is a well-scoped opening contribution. Until then this project keeps its own template validator.

## Risks accepted

Recorded plainly because they are the reason this is a decision rather than a default.

- **Bus factor of one.** FerroEHR has a single maintainer, no organisation, and no legal entity behind it; the project states this itself in `MAINTAINERS.md` and treats it as a finding rather than a footnote. It is a serious dependency risk on a project that is nonetheless unusually rigorous about disclosing it.
- **`0.0.x` versioning.** No semver stability promise, and the published crates already run ahead of the in-repo workspace version. Pin exactly and expect churn.
- **MSRV rose to 1.96** at Stage 1, set by the crates themselves.
- **Dependency footprint grows** (`rust_decimal`, `chumsky`, `logos`, `indexmap`, `roxmltree`, `serde_jcs`, `serde_path_to_error`, `stacker`, and more) against a project whose distinguishing claim is a light single binary with no runtime. Stage 1 alone cost 14 crates and +13% binary size. Measure before and after each stage; the single-binary promise is about not shipping a JVM or a database server, not about a small dependency tree, but the trade should be observed rather than ignored.

The mitigation for the first three is a fork from the last permissive version, which needs nobody's permission. The fourth risk - relicensing - has now materialised, and is recorded above rather than here because it changed the plan rather than merely threatening it.

## Licensing

**The crate set was relicensed away from MIT after 0.0.56, and the result is a split that decides which stages remain open.** Everything at or below 0.0.56 is MIT; 0.0.58 moved to Apache-2.0; from 0.0.60 (2026-09-04) some crates became BUSL-1.1 under Vernum Projecten B.V. The repository itself is now BUSL-1.1.

| Crate | Licence at 0.0.67 | Usable by anarchie |
|---|---|---|
| `openehr-base`, `openehr-rm`, `openehr-am`, `openehr-lang` | Apache-2.0 | Yes |
| `openehr-term` | `Apache-2.0 AND CC-BY-SA-3.0` | Yes, with the data attribution |
| `openehr-query`, `openehr-adl` | BUSL-1.1 | **No** |
| `openehr-its` | `BUSL-1.1 AND Apache-2.0` | **No** - `AND` means both apply |

BUSL-1.1 is source-available, not open source. Its Additional Use Grant permits production use for non-commercial purposes only, forbids offering the work as a hosted, managed, or embedded service that stores or queries health data for third parties, and forbids distributing it for a fee. Each version converts to Apache-2.0 four years after publication.

That is incompatible with this project in two independent ways. AGPL-3.0-or-later forbids imposing further restrictions on downstream recipients, and a field-of-use restriction is exactly such a restriction; and anarchie ships prebuilt binaries and publishes to crates.io, which the grant limits directly. A BUSL crate therefore cannot enter the dependency graph while anarchie is AGPL.

Apache-2.0 composes into AGPL-3.0-or-later one way, so the Stage 2 crates remain available and the main prize is untouched. The `CC-BY-SA-3.0` terminology bundle in `openehr-term` is a redistribution obligation that sits naturally in the existing four-layer split alongside the CKM archetype derivatives already carried at 3.0; record it in [licensing.md](licensing.md) and keep `reuse lint` green when Stage 2 lands.

`openehr-query` stays pinned at 0.0.56 under its irrevocable MIT grant, and Dependabot is configured to ignore it so a bump cannot land by routine merge. The pin is safe indefinitely but it is frozen: upstream parser fixes no longer reach us.

**The maintainer confirmed this is permanent, not transitional.** `openehr-query` is his own hand-written lexer/parser/AST, which the stated rule places on the BUSL side by design; `openehr-adl` likewise. Neither is expected to return to a permissive licence. The practical consequence: if AQL 1.1 coverage needs to grow beyond what 0.0.56 already parses, the choice is between living with the frozen subset and maintaining a fork of it under its own MIT terms - not waiting for a future upstream release to rescue us. No fork has been started; this records the decision point for when the frozen subset first becomes a real limitation, rather than pre-emptively duplicating a crate we can still depend on cleanly.

## GitEHR

The earlier plan made GitEHR the trigger for extracting an `anarchie` SDK. It is now simpler: GitEHR consumes the same `openehr-*` crates directly. Nothing has to be extracted, published, or version-managed by this project first, and the two products share a model layer without either depending on the other.

The shared conformance obligation survives the change. The first GitEHR integration should still add positive and negative cases proving that the same canonical Composition parses, serialises byte-stably where the profile guarantees it, and validates equivalently in both products.

## Reversal

If FerroEHR becomes unmaintained, diverges from the specification, or takes a direction this project cannot follow, the exit is to fork from the last permissively licensed version and continue. For the Apache-2.0 crates that is the current release; for the BUSL crates it is 0.0.56, the last MIT one. Those grants are irrevocable and cannot be withdrawn retrospectively, so the exit exists in every case - but for a BUSL crate it starts from a frozen 2026-09-02 snapshot rather than from upstream head.

The relicensing is the reason this section is no longer hypothetical. It also dates the risk register above: the "bus factor of one, no legal entity" reading was accurate when written and `MAINTAINERS.md` still asserts it, but a BUSL licensor (Vernum Projecten B.V.) now exists. Re-read the project's own governance documents before relying on them.

Asked directly whether the split was deliberate, the maintainer answered same-day with a stated, principled rule (generated code stays Apache-2.0; hand-written code is BUSL-1.1) rather than an unexplained or defensive response, and volunteered a further concession - splitting `openehr-its` to move its generated half back to Apache-2.0 - in the same reply. That is a better governance signal than the relicensing alone suggested: whoever controls the licensing decision is engaging with downstream consumers and adjusting scope in response, not simply asserting new terms. It does not resolve who legally controls the decision or whether it can be reversed on request, only that it is being made thoughtfully and is open to negotiation at the margins.
