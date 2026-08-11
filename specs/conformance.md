# Conformance Programme

## Decision

`anarchie` will claim conformance only to named, versioned, executable profiles. "openEHR compliant" without a declared Reference Model release, serialisation, API surface, AQL feature set, template generation, validation policy, and evidence set is not a useful claim.

Conformance is a continuing product programme, not a final test added after implementation. Every implementation surface has a machine-readable case corpus, provenance, pinned reference material, and an independent comparison where one is available. The shipped Rust runtime remains independent of all JVM and server oracles.

## Conformance Matrix

The project reports conformance by dimension:

| Dimension | Contract | Current evidence | Major remaining evidence |
|---|---|---|---|
| RM representation | Named openEHR RM release and supported type inventory | Canonical JSON round-trip tests and [reference-model-coverage.md](reference-model-coverage.md) | Wider externally sourced RM vectors and deferred types |
| Canonical serialisation | Parse, canonicalise, and reparse without semantic or byte drift | Idempotence and shared fixture tests | Official/external canonical JSON corpus and XML equivalence |
| RM validation | Required attributes, invariants, data-value validity | Native tests and initial Archie differential | Broad RM class/invariant matrix |
| OPT validation | Existence, occurrences, cardinality, primitive constraints, terminology constraints, slots | Native tests and six Archie differential verdicts | Nested archetypes, slot resolution, rules, broader constraints |
| Template ingestion | Accepted source/template formats and deterministic generated artefacts | Native flattened JSON OPTs | ADL 1.4 OPT XML, OET pipeline, ADL2/OPT2 policy |
| AQL syntax | Supported grammar profile | Shared official-grammar corpus | Wider grammar corpus and explicit unsupported cases |
| AQL semantics | Result meaning over shared datasets | Twenty-three EHRbase differential cases | Multiple Compositions/EHRs, nested containment, functions, temporal/version queries |
| REST API | Operations, methods, headers, statuses, media types, preconditions | Native integration tests | Endpoint-by-endpoint differential suite against a reference CDR |
| Renderer formats | Canonical JSON/XML, FLAT, STRUCTURED, Web Template transformations | Canonical JSON only | Round-trip corpora and reference renderer comparison |
| Knowledge packages | Resolution, provenance, reproducibility, safe installation | Starter pack tests | Manifest/lock suite, CKM snapshot closure, reproducible package builds |
| Terminology | Structural bindings and optional external validation policy | Structural code checks | Backend contract and licensed value-set evidence |

The matrix is the basis for release notes and machine-readable conformance reports. A green row means conformance to its declared subset, not necessarily complete implementation of every upstream feature.

## Evidence Ladder

Evidence gets stronger in this order:

1. **Internal unit test:** confirms implementation intent but can repeat the implementation's own mistaken assumption.
2. **Specification-derived vector:** cites a normative example or rule and records the expected result.
3. **External fixture:** exercises real artefacts from CKM, SDKs, CDRs, or published examples with provenance and licence recorded.
4. **Differential oracle:** runs the same semantic case through an independent implementation.
5. **Cross-oracle agreement:** compares more than one independent implementation where versions and policy align.
6. **Real-client interoperability:** exercises an existing form renderer, SDK, or application against anarchie's public interface.

No oracle is treated as the specification itself. A disagreement enters investigation with four live hypotheses: anarchie defect, oracle defect, version/policy mismatch, or unspecified behaviour.

## Case Model

Every conformance case should record:

```json
{
  "id": "opt-wrong-category-code",
  "dimension": "opt-validation",
  "profile": "rm-1.1.0+adl2-opt2",
  "input": "fixtures/wrong-category-code.json",
  "template": "fixtures/anarchie-validator.adls",
  "expected": {
    "valid": false,
    "category": "terminology-constraint"
  },
  "provenance": {
    "source": "synthetic mutation of blood-pressure-composition.json",
    "licence": "AGPL-3.0-or-later"
  }
}
```

The first contract is semantic verdict. Mature cases add stable error category, RM path, archetype path, severity, response status, or normalised result data as appropriate. Exact human wording and irrelevant JSON ordering are not conformance contracts.

Case identifiers are stable. Cases are changed only for an intentional profile change or corrected evidence, with the rationale reviewed alongside the expected-output diff.

## Corpus Structure

Test-owned conformance remains under `tests/` while each dimension is small:

```text
tests/
|-- archie/
|   |-- README.md
|   |-- cases.json
|   `-- fixtures/
|-- aql-conformance/
|   |-- README.md
|   `-- cases.tsv
|-- ehrbase/
|   |-- README.md
|   `-- queries.json
`-- conformance/
    |-- profiles/
    |-- fixtures/
    |-- expected/
    `-- provenance/
```

The cross-cutting `tests/conformance/` root should be introduced once shared profiles, fixtures, or reporting would otherwise be duplicated. Existing dimension-specific harnesses are retained and can consume the shared manifests.

## Profiles

A conformance profile freezes the assumptions needed to interpret results:

- openEHR RM release;
- ADL/AOM/OPT generation;
- JSON/XML or renderer format version;
- AQL grammar/specification revision and supported subset;
- REST API revision and supported operations;
- terminology-validation policy;
- oracle implementation version/revision and configuration;
- fixture package/lock digest.

Examples:

```text
rm-1.1.0-canonical-json
rm-1.1.0-adl2-opt2-validation
aql-1.0-anarchie-subset-1
its-rest-composition-subset-1
knowledge-package-format-1
```

An ADL 1.4 OPT result and an ADL2/OPT2 result do not belong to the same validation profile merely because both are called an Operational Template.

## Oracle Roles

### Archie

Archie is the independent RM and ADL2/OPT2 validation oracle. It is source-pinned and called through `s/archie-validate` and `s/archie-conformance`. The current corpus has six root Composition cases. Nested archetype validation requires a complete repository rather than treating `ARCHETYPE_NOT_FOUND` as a data verdict.

### EHRbase

EHRbase is the primary REST and AQL semantic oracle and an important ADL 1.4 ecosystem reference. It runs locally in a pinned disposable Docker environment. Its output is normalised before semantic comparison; database-specific ordering or representation is not adopted as a contract unless the specification requires it.

### Official Grammars and Schemas

The official AQL grammar is a syntax oracle, not a semantic oracle. Official schemas can establish structural validity but do not prove behavioural equivalence. Revisions and downloaded artefacts are pinned and checksummed.

### Real Clients

Form renderers and SDKs become acceptance oracles for Web Template, FLAT, STRUCTURED, content negotiation, and REST usability. These tests should exercise public interfaces without importing client implementation assumptions into anarchie's core.

## Lessons Already Established

- The published `openEHR/specifications-CNF` repository contains conformance schedules and platform tests, not a comprehensive ready-to-run validator-vector corpus. Cases need source-by-source provenance.
- Reference implementations target different generations. Archie 3.17 is ADL2/OPT2-oriented, while the EHRbase blood-pressure fixture is legacy ADL 1.4 OPT XML.
- A fixture accepted by one implementation may omit data another considers mandatory. Establish a baseline valid under every compared implementation before creating mutations.
- Parsing failure, RM structural failure, RM invariant failure, OPT failure, unresolved archetype, and terminology-backend failure are distinct outcomes.
- Syntax agreement does not establish semantic agreement. AQL requires both official-grammar and shared-result testing.
- Differential testing finds boundary assumptions that self-tests miss. The first Archie corpus exposed skipped `C_CODE_PHRASE` validation when monomorphic `DV_CODED_TEXT` lost its `_type` tag during anarchie's internal projection.
- Verdict agreement is the first rung, not the finish. Error category and path agreement provide stronger evidence; exact prose is generally too brittle.
- Pinned versions, revisions, compiler configuration, source fixtures, and package locks are part of the test input.
- Byte-stable canonical JSON is a valuable anarchie invariant but is not by itself proof of openEHR interoperability.

## Investigation Workflow

When a differential case disagrees:

1. Preserve both complete machine-readable outputs and environment revisions.
2. Confirm both sides parsed the same semantic input and resolved the same template/archetype versions.
3. Classify the stage: parse, RM, invariant, OPT, terminology, query, HTTP, or representation.
4. Reduce to one controlled fixture mutation where possible.
5. Consult the normative specification and record the relevant version and citation.
6. Add another implementation or schema check if the rule remains ambiguous.
7. Fix anarchie only when evidence supports it; otherwise document the oracle/profile difference.
8. Retain the reduced case as a regression vector.

## Release Reporting

Every release should eventually emit a machine-readable conformance report containing:

- profile identifiers;
- case counts by dimension and evidence level;
- pass, expected limitation, disagreement, and skipped counts;
- exact anarchie revision;
- oracle versions/revisions and configuration;
- knowledge lock/package digests;
- known unsupported surface;
- links to disagreement records and normative evidence.

The human release summary should state supported subsets and residual risks. It must not convert partial evidence into an unqualified certification claim.

## Scope Boundary

Conformance remains part of the anarchie product because it defines whether the CDR can honestly claim interoperability. Oracle implementations remain outside the shipped runtime. The Rust core exposes stable operations that thin test adapters exercise; no production code should branch on the presence of Archie, EHRbase, Docker, Java, or network access.

The corpus may eventually deserve an independent repository if multiple openEHR implementations adopt it or its release cadence separates from anarchie. Until then, keeping cases beside the implementation makes behavioural changes and evidence updates reviewable together. The extraction trigger is a real second consumer, not corpus size.

## Roadmap

Legend: `[x]` done, `[~]` partial, `[ ]` not started.

- [~] **C1 - Declare profiles.** Current specs document subsets and versions in prose; formal machine-readable profile manifests remain to be added.
- [~] **C2 - RM and canonical JSON corpus.** Internal round-trip, invariant, and fixture tests exist; externally sourced RM vectors and broader type coverage remain open.
- [~] **C3 - Archie validation differential.** Six root Composition verdicts run against pinned Archie and have already exposed one native validation defect; nested repositories and wider constraint classes remain open.
- [~] **C4 - AQL syntax differential.** The supported corpus is checked against the pinned official grammar; expand positive and negative cases with feature ownership.
- [~] **C5 - EHRbase AQL semantics.** Twenty-three cases compare shared query results; add multi-EHR datasets, nested containment, functions, temporal/version queries, and explicit known differences.
- [ ] **C6 - REST differential.** Build an operation/header/status/media-type matrix and execute the same lifecycle against anarchie and pinned EHRbase.
- [ ] **C7 - Template and renderer conformance.** Add ADL 1.4 ingestion, OPT generation, Web Template, FLAT, STRUCTURED, canonical XML, and real-client round trips.
- [ ] **C8 - Knowledge-package conformance.** Test deterministic resolution, lock reproduction, malicious archives, dependency failures, updates, rollback, provenance, and licence gates.
- [ ] **C9 - Terminology contract.** Define structural-only and backend-enabled profiles; compare permitted-code/value-set behaviour without bundling licensed terminology content.
- [ ] **C10 - Conformance report.** Generate per-release JSON and human summaries from all dimension harnesses.
- [ ] **C11 - External corpus contribution.** Publish reusable cases or extract the corpus when a second implementation commits to consuming it.

## Completion Criteria

The conformance programme reaches its first complete milestone when:

- every shipped surface maps to a named profile and case manifest;
- every claimed feature has at least one positive and one targeted negative case;
- external artefacts and expected outputs carry provenance and licence metadata;
- all oracles and official resources are pinned and reproducible;
- validation, AQL, and REST have independent semantic comparisons;
- package and renderer outputs are reproducible from `knowledge.lock`;
- releases publish machine-readable evidence and explicit limitations;
- no test-time oracle or heavyweight runtime enters the shipped binary.
