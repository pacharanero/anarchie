<!--
SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# AQL, ADL, and ANTLR: should anarchie adopt generated parsers?

Evaluation of [Issue #9](https://github.com/pacharanero/anarchie/issues/9) and the broader question of parser generation for anarchie, informed by inspecting EHRbase, archie, and the Rust parser-generator ecosystem.

## What the issue proposed

`tinovyatkin` proposed generating a Rust AQL parser from the official openEHR ANTLR grammar (`AqlLexer.g4` + `AqlParser.g4` in `openEHR/specifications-QUERY`) using their clean-room `antlr-rust-runtime`, instead of growing anarchie's hand-written 783-line lexer/parser/AST.

Their honest caveats: (1) the generated tree shape differs from anarchie's current `AqlQuery` AST, so the executor front-half needs adapting; (2) it only pays off if closing the gap to full AQL is actually a goal.

## What we found by inspecting the reference implementations

### EHRbase does NOT parse AQL itself - and does NOT own a grammar

EHRbase delegates AQL parsing entirely to the **openEHR SDK** (`org.ehrbase.openehr.sdk:aql`), a separate Maven artefact pulled in as a dependency. The ehrbase repo contains:

- **Zero** `.g4` grammar files
- **Zero** `org.antlr` imports in Java source
- **No** ANTLR plugin in any `pom.xml`

The SDK's `aql` module contains the ANTLR grammar (derived from the official openEHR one, same authors: Sebastian Iancu, Teun van Hemert, Thomas Beale), generates a parser at build time via `antlr4-maven-plugin`, and exposes `AqlQueryParser.parse(String) -> AqlQuery` as the public API. EHRbase's `AqlQueryRequest.java:62` calls that single method.

ADL parsing is similarly delegated: EHRbase pulls in **archie** (`com.nedap.healthcare.archie:grammars`) transitively via the SDK's validation/serialisation modules for template handling. It never parses ADL itself.

The lesson: the reference Java CDR treats parsing as a library concern, not an application concern. It does not own the grammar; it depends on a shared ecosystem library that does.

### archie IS the grammar host - and uses ANTLR for ADL, not just AQL

archie (Nedap's Java openEHR library, the de facto reference implementation) takes the opposite approach: it **owns and builds the grammars directly**.

`archie/grammars/src/main/antlr/` contains 21 `.g4` files covering:

- **ADL2**: `Adl.g4` (imports `cadl.g4` + `odin.g4`), plus `adl_rules.g4`, `cadl_primitives.g4`, `adl_keywords.g4`, `BaseLexer.g4`, `base_patterns.g4`, `odin_values.g4`, `ContainedRegex.g4`
- **ADL 1.4**: a parallel `*14.g4` family (`Adl14.g4`, `cadl14.g4`, `odin14.g4`, etc.)
- **AQL**: `AqlLexer.g4` + `AqlParser.g4` (the official openEHR AQL grammar, "optimized for use by Nedap" per a comment in `AqlParser.g4:18`)
- **XPath**: `XPath.g4` (third-party, by Jan-Willem van den Broek)
- **ODIN**: standalone `odin.g4` + `odin_values.g4` (also embedded in the ADL composition)

The Gradle ANTLR plugin generates Java parser/lexer/listener/visitor classes at build time (not checked in; `.gitignore` excludes `grammars/gen` and `grammars/build/`). ANTLR version: **4.13.2**.

Critically, archie's architecture is:

```
.g4 grammar -> ANTLR-generated parse tree -> Listener/Visitor tree-walk -> AOM semantic objects
```

The ANTLR parse tree **never leaks** past the `aom` module boundary. Downstream code works with `Archetype`, `CComplexObject`, `CArchetypeRoot`, etc. - proper domain objects. This is exactly the "semantic AST adapter" boundary I recommended in the earlier evaluation.

One surprise: **archie does not consume the AQL grammar it hosts**. There are zero Java imports of `AqlLexer`/`AqlParser`/`AqlBaseListener` anywhere in archie. The AQL grammar was moved into archie in January 2024 (commit by `eline.brader@nedap.com`) as a hosting arrangement for downstream Nedap projects. archie's own AQL semantic layer does not exist - the grammar is compiled but unwired.

### The official grammar landscape

There are actually **three** repos publishing the AQL/ADL grammar, all sharing the same author lineage (Thomas Beale, Sebastian Iancu, Teun van Hemert):

1. **`openEHR/specifications-QUERY`** - the normative spec repo (`docs/AQL/grammar/AqlLexer.g4` + `AqlParser.g4`). This is what Issue #9 referenced.
2. **`openEHR/adl-antlr`** - the "ANTLR Grammar Forge for ADL" (Apache 2.0, 2 stars, 80 commits). Covers ADL2 only (ODIN + ADL + cADL + RULES + PCRE regex). Its README states the grammars are "lightly tested" and "by no means comprehensively tested", and notes an ongoing harmonisation effort with the `adl2-core` grammars.
3. **`archie/grammars`** - the production-grade, fully-wired grammars (ADL2 + ADL 1.4 + AQL + XPath). This is what EHRbase uses transitively. The AQL grammar here is the same openEHR Foundation grammar, Nedap-optimised.

The grammars are **not identical across repos** - they are derivatives sharing the same lineage. The SDK's version uses `selectQuery` as the start rule; archie's uses `query`. Both claim to accept the same language. This is the reality of the openEHR grammar ecosystem: there is a canonical lineage, not a canonical file.

## Answering the five questions

### 1. Do we do this same .g4 process for ADL as well as AQL?

**For anarchie today: no, because anarchie deliberately does not parse ADL.**

anarchie's design (see `src/aom.rs:1-15`) explicitly avoids parsing raw ADL archetypes. It validates Compositions against *flattened Operational Templates* (OPT), and flattening "is the job of upstream tools (Archetype Designer, ADL Workbench, Archie)". anarchie ingests pre-flattened templates as JSON, not ADL text.

So the ADL grammar question is moot for anarchie unless the project takes on archetype flattening - a large, separate undertaking. If it ever does, the answer would be yes: ADL is a much harder language to hand-parse than AQL (composite grammars, mode-switching lexer, regex sub-grammar, ODIN embedded format), and the official grammar exists and is proven (archie uses it in production). But that is not the current scope.

The AQL question stands on its own. AQL and ADL are independent decisions, and conflating them would be a mistake.

### 2. Are there other versions of this "AST language stub generator" approach? (eg LLVM?)

ANTLR is one member of a broader family of parser generators. The relevant alternatives for Rust are:

| Tool | Stars | Maturity | Grammar format | Rust-native? | Build dependency |
|---|---|---|---|---|---|
| **LALRPOP** | 3,500 | High (1,651 commits, 40 releases, used by RustPython, Gluon, Solang) | `.lalrpop` (custom DSL) | Yes - Rust build-time macro/codegen | `lalrpop` crate as build-dep; no runtime dep |
| **pest** | ~4k | High (used in many Rust projects) | `.pest` (PEG grammar) | Yes - Rust build-time macro | `pest` + `pest_derive` crates |
| **tree-sitter** | 26,200 | Very high (6,409 commits, 101 releases, used by GitHub, Neovim, Zed) | `.js` grammar (JS DSL generating C) | Runtime is C with Rust bindings | C runtime + tree-sitter CLI (Node.js) |
| **nom** | ~3k | Very high (parser combinator, not generator) | Rust code directly | Yes - pure Rust | `nom` crate only |
| **chumsky** | ~2k | Medium-high (parser combinator) | Rust code directly | Yes - pure Rust | `chumsky` crate only |
| **ANTLR + antlr-rust-runtime** | 6 | **Very low** (75 commits, 17 releases, <2 months old) | `.g4` (ANTLR DSL) | Runtime is Rust, but generation needs Java ANTLR tool | Java toolchain + `antlr-rust-runtime` crate |
| **ANTLR + antlr-rust (IGGGIT)** | ~? | Low (older fork, less active) | `.g4` | Rust runtime | Java toolchain + crate |

**LLVM is not a parser generator.** LLVM does not generate parsers from grammar files. It is a compiler IR and codegen infrastructure. The equivalent in the parser space would be the tools above. If the question was "is there a grand unified infrastructure like LLVM for language front-ends", the answer is: tree-sitter is the closest (it provides a unified grammar format, a C runtime, and bindings for many languages), but it is optimised for *incremental* parsing in editors, not for one-shot query parsing, and its grammars are written in JavaScript, not in a declarative DSL.

### 3. Is openEHR spec published in any of these alternative formats as well as .g4?

**No.** The openEHR specifications publish grammars **only** in ANTLR v4 `.g4` format (and historically in ANTLR v3 and yacc/lex, both superseded). There is no official openEHR grammar in LALRPOP, pest, tree-sitter, or any other format. The `openEHR/adl-antlr` README confirms the grammars are "derived from the older ADL2 reference grammars in yacc/lex" but the current publication target is ANTLR4 only.

This means any non-ANTLR approach requires **translating** the `.g4` grammar into another format. That translation is a one-time manual effort per grammar revision, and the result is not officially maintained - you own the divergence.

With agentic coding, that translation is cheap to generate but must be verified. The `.g4` grammar is the conformance oracle; your translated grammar is a derivative that could drift.

### 4. antlr-rust-runtime is not even 2 months old. Are there other more mature projects?

**There is one older Rust ANTLR runtime**: `antlr-rust` by `IGGGIT` (the crate referenced in archie's own earlier exploration, and the one the `specs/query-engine.md:99` note likely refers to). It is a port of the Java ANTLR4 runtime to Rust. However, it has been largely unmaintained for years, has known issues with newer ANTLR versions, and the new `ophi-dev/antlr-rust-runtime` was created specifically as a clean-room replacement because `antlr-rust` was not viable.

So the Rust ANTLR runtime landscape is:

- **`ophi-dev/antlr-rust-runtime`**: 6 stars, 75 commits, <2 months old, but passes the full ANTLR conformance suite (357/357), claims 1.8x-18x faster than Go ANTLR, BSD-3, pure Rust runtime, needs Java ANTLR tool for generation. Single maintainer (`tinovyatkin`, the issue author).
- **`IGGGIT/antlr-rust`**: older, largely unmaintained, known compatibility gaps with ANTLR 4.10+.

**There is no mature, widely-adopted, multi-maintainer ANTLR runtime for Rust.** The official ANTLR project supports Java, C#, Python, JavaScript, Go, C++, Swift, Dart, and PHP as first-class targets. Rust is **not** an official ANTLR target. Both Rust runtimes are third-party community efforts.

This is a significant supply-chain consideration. For a project that may become a credible CDR, depending on a 6-star, single-maintainer, <2-month-old runtime for a critical-path parser is a real risk - regardless of code quality.

### 5. The "paper over the madness" philosophy

This is the most important point, and it deserves a direct answer.

openEHR's technical history includes:
- Eiffel as the original implementation language (closed-source, no CI possible, effectively dead as a community tool)
- ADL syntax that is genuinely difficult (nested archetype specialisation, `#` path predicates, regex sub-grammar, ODIN format embedded in ADL, two major grammar versions)
- A grammar ecosystem spread across 3 repos with derivatives, not a single canonical file
- "Lightly tested" grammars in the official forge repo

The Rust implementation was partly motivated by being able to "paper over" this, not "drink the koolaid". Adopting ANTLR would be the opposite: it means adopting openEHR's own chosen parsing infrastructure, on openEHR's terms, with all the ecosystem complexity that entails.

**The counter-argument** (and it's a strong one): if anarchie's goal is community credibility, then *not* using the same grammar as everyone else is the risk. The openEHR community has already decided that ANTLR `.g4` is the grammar format. EHRbase uses it (via the SDK), archie uses it, Better uses it. A Rust CDR that hand-rolls its own parser is, by definition, running a private dialect - even if it's a faithful one. "Papering over the madness" by reimplementing the parser means anarchie's AQL coverage claims are unverifiable against the reference grammar without a separate conformance harness.

**The resolution** depends on what anarchie is optimising for:

| If anarchie is... | Then... |
|---|---|
| A teaching/learning project | Hand-written parser is pedagogically superior; fetch a pinned `.g4` as a conformance oracle only |
| A credible production CDR | Parser fidelity to the spec is table stakes; community trust requires verifiable conformance, not "we reimplemented it faithfully, trust us" |
| The fastest openEHR CDR (the sct precedent) | Parser performance matters; a hand-written parser gives full control and is probably faster, but only matters if parsing is on the hot path (it likely isn't - execution dominates) |

## The recommendation, revised

Given the `sct` precedent and the stated willingness to evolve toward a full CDR, my recommendation shifts from the earlier "decline for now" to:

### Short term: pin the grammars externally, keep the hand-written parser, add a conformance harness

1. **Record a pinned official grammar revision and its checksums** (from `openEHR/specifications-QUERY` for AQL). Fetch it only into a temporary directory for the conformance-oracle run, rather than redistributing CC-BY-ND specification material in this repository.
2. **Keep the hand-written Rust parser** as the production parser. It is 783 lines, zero-dependency, and covers the current subset.
3. **Build a conformance test harness** that runs anarchie's parser against a corpus of AQL queries derived from the grammar (and from real-world examples). This gives verifiable coverage claims without a runtime dependency.
4. **Accept the PoC offer as a spike** - have `tinovyatkin` generate a Rust parser from the official grammar, evaluate the tree shape, the build dependency, and the generated code quality. This is information-gathering, not a commitment.

### Medium term: decide based on evidence, not aspiration

The decision to adopt a generated parser should be made when you hit a concrete wall:
- You need to accept AQL constructs that are painful to hand-parse (complex `CONTAINS` nesting, full path predicate grammar, functions)
- The hand-written parser is demonstrably diverging from the spec in ways that matter to users
- The ANTLR Rust runtime has matured (more maintainers, wider adoption, proven in other projects)

If all three are true, the case for generation is strong. If none are true, it is architecture astronautics.

### The ADL question is separate and should stay deferred

anarchie's decision to ingest flattened OPT rather than parse ADL is architecturally sound. Do not adopt ANTLR for ADL unless you decide to take on archetype flattening, which is a much larger decision than parser technology.

### What not to do

- Do not adopt a 6-star, single-maintainer runtime as a critical-path dependency without evidence it will be maintained long-term
- Do not adopt ANTLR (any runtime) if the no-build-step promise is non-negotiable - ANTLR needs the Java tool for generation, and vendoring generated code is a maintenance smell
- Do not hand-translate the `.g4` into LALRPOP/pest/tree-sitter and claim conformance - the translation is a derivative that needs its own verification
- Do not conflate the AQL and ADL decisions

## Summary table

| Question | Answer |
|---|---|
| Same .g4 process for ADL? | No - anarchie doesn't parse ADL; it ingests flattened OPT. ADL parsing is a separate, larger decision. |
| Other "AST stub generators" (LLVM)? | LALRPOP, pest, tree-sitter, nom, chumsky exist. None is "like LLVM". openEHR publishes only `.g4`. |
| openEHR spec in alternative formats? | No - ANTLR v4 `.g4` only. Any other format requires translation (a derivative). |
| More mature Rust ANTLR runtimes? | One older unmaintained one (`IGGGIT/antlr-rust`). No mature, multi-maintainer Rust ANTLR runtime exists. |
| Should we adopt ANTLR? | Not yet. Vendor the `.g4`, keep hand-written parser, add conformance harness. Revisit when you hit a wall. |
