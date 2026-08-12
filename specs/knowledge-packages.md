# Knowledge Packages

## Decision

`anarchie` will be a batteries-included CDR, not an empty template server. A default offline installation should contain the complete eligible published openEHR International archetype library, a curated catalogue of executable templates, examples, provenance, and an immutable dependency lock. Operators can add, update, inspect, vendor, and remove further knowledge packages through the same `anarchie` CLI.

The package manager is part of `anarchie` because installed clinical knowledge directly determines which Compositions the CDR accepts. Package compilation and upstream harvesting are build-time concerns; they must not introduce a JVM, network service, or other runtime dependency into the shipped binary.

This subsystem is the **Knowledge Artefacts Manager (KAM)**. KAM is its architectural name, not a second executable: users operate it through `anarchie knowledge`, `anarchie archetype`, `anarchie template`, and `anarchie pack`. The earlier `knowledge-artefacts-package-manager` repository remains useful prior art, but this specification is the source of truth for anarchie's implementation.

This extends the existing starter templates and local-directory packs described in [bundled-archetypes.md](bundled-archetypes.md). It does not replace the core invariant that canonical Composition JSON is the clinical system of record. Knowledge sources and generated templates are schema inputs, versioned separately from patient data.

## Product Promise

The target first-run experience is:

```sh
anarchie init my-cdr
cd my-cdr
anarchie archetype list
anarchie template list
anarchie template example vital_signs_encounter.v1 > composition.json
anarchie validate composition.json --template vital_signs_encounter.v1
```

`anarchie init` must work offline and install a verified embedded knowledge snapshot. `anarchie init --minimal` remains the explicit empty-CDR option. Network access is only needed when the operator asks to discover or update packages.

## Meaning of Complete

"Complete" has three deliberately separate meanings:

1. **Complete published library:** every openEHR International archetype eligible under the package policy, together with its complete hard dependency closure, original source, metadata, provenance, licence determination, and content digest.
2. **Complete executable catalogue:** a broad but curated set of templates that compile reproducibly and have examples and independent conformance evidence. Archetypes alone do not make a CDR usable; executable templates do.
3. **Complete authoring trunk:** all draft and unstable upstream artefacts, including unresolved material. This is useful for modelling work but is opt-in and must never be confused with the supported default.

The default package is the complete published library plus the supported executable catalogue. It is not a blind copy of a moving authoring trunk.

## CKM Mirror Assessment

The intended primary upstream is the [openEHR International CKM GitHub mirror](https://github.com/openEHR/CKM-mirror). It is suitable source material but not itself a package registry or release channel.

A point-in-time inspection of commit `72f5a0828145ddb3b19cbfdafef79ba3bcfcf6f2` on 11 August 2026 found:

| Property | Observed value |
|---|---:|
| Archetypes | 689 ADL 1.4 files |
| International `local/` archetypes | 659 |
| Published archetypes | 232 |
| In-development archetypes | 455 |
| OET source templates | 44 |
| Published OET templates | 0 |
| Generated OPTs | 0 |
| Source asset size | Approximately 20.4 MiB |

Important source limitations:

- The mirror is an automatically updated authoring trunk and has no useful release series. Package builds must pin a Git commit.
- Lifecycle state, not a filename such as `.v1`, determines whether an archetype is published.
- Three specialised archetypes in the inspected snapshot have missing hard parents.
- Archetype slots are usually regular-expression constraints, not direct dependencies. A slot must remain symbolic until a template chooses a filler.
- Thirty-eight of the forty-four OET templates have at least one exact archetype reference unavailable in the snapshot. Across those templates, 183 unique referenced IDs are absent. Major-version substitution would be unsafe.
- Template filenames and lifecycle metadata do not supply a dependable release version. Package releases need their own version and immutable digest.
- Repository-level licensing says CC-BY-SA-3.0, while many current artefact metadata blocks say CC-BY-SA-4.0 and some omit a licence. Every package must preserve source declarations and record the evidence used for its licence determination; it must not flatten this into an unsupported blanket claim.
- The single termset is an external SNOMED query, not redistributable terminology content. Packages may contain bindings and queries but not terminology content.

The source audit is evidence for package policy, not a permanent statement about the moving mirror. Every package release regenerates the inventory and records its own counts and exclusions.

## Ubiquitous Language

| Term | Definition |
|---|---|
| **Knowledge artefact** | One archetype, source template, Operational Template, Web Template, termset query, stored query, mapping, example, or conformance fixture. |
| **Knowledge package** | A named, versioned, immutable collection of knowledge artefacts plus package dependencies and provenance. |
| **Knowledge manifest** | The human-authored declaration of desired packages, active templates, registries, languages, and policy. Stored as `knowledge.toml`. |
| **Knowledge lock** | The generated complete resolution, including exact package versions, source revisions, artefact revisions, dependency edges, checksums, licences, and compiler identities. Stored as `knowledge.lock`. |
| **Registry** | An index of immutable knowledge-package releases. A registry is discovery metadata, not the authoritative source for installed content. |
| **Upstream source** | CKM or another repository from which a package publisher harvests source artefacts. |
| **Installed knowledge base** | The content-addressed package material available to one anarchie deployment. |
| **Active template** | The selected template digest used for new validation and example generation. |
| **Template digest** | The immutable content identity of the exact template used to validate a Composition. |
| **Hard dependency** | A dependency that must resolve for the artefact to compile or operate, such as a specialisation parent or an explicitly selected template component. |
| **Slot constraint** | A symbolic rule describing permitted archetype fillers. It is not a hard dependency until a template selects a filler. |

## Knowledge Manifest

The deployment declaration is `knowledge.toml`. It expresses intent and policy, not a fully resolved file list.

```toml
[knowledge]
name = "anarchie-default"
rm-release = "1.1.0"
default-language = "en"

[registries.ckm]
type = "git"
url = "https://github.com/openEHR/CKM-mirror.git"

[dependencies]
ckm-international = "^2026.8"
anarchie-clinical-templates = "^1.0"
anarchie-ips = "^1.0"

[policy]
allowed-lifecycle-states = ["published"]
allow-drafts = false
allow-missing-hard-dependencies = false
allow-unresolved-slots = true
allowed-licences = ["CC-BY-SA-3.0", "CC-BY-SA-4.0"]
require-attribution = true
require-example-compositions = true
require-conformance-evidence = true

[languages]
include = ["en"]

[templates]
activate = [
  "vital_signs_encounter.v1",
  "problem_list.v1",
  "medication_list.v1",
  "adverse_reaction_list.v1",
  "laboratory_result_report.v1",
  "immunisation_list.v1",
  "procedure_list.v1",
  "encounter_note.v1",
]
```

The manifest may declare a specialist or national package, but non-International sources are never silently admitted to the default policy. Licence acceptance and lifecycle policy remain explicit per deployment.

## Knowledge Lock

`knowledge.lock` is generated, committed, and authoritative for resolution. Two clean machines resolving against the same registry state must produce the same lock.

```toml
version = 1

[[package]]
name = "ckm-international"
version = "2026.8.1"
source = "git+https://github.com/openEHR/CKM-mirror.git"
revision = "72f5a0828145ddb3b19cbfdafef79ba3bcfcf6f2"
checksum = "sha256:..."

[[artefact]]
id = "openEHR-EHR-OBSERVATION.blood_pressure.v2"
revision = "2.0.1"
kind = "archetype"
lifecycle = "published"
path = "local/archetypes/entry/observation/openEHR-EHR-OBSERVATION.blood_pressure.v2.adl"
checksum = "sha256:..."
licence = "CC-BY-SA-4.0"
package = "ckm-international@2026.8.1"

[[artefact]]
id = "vital_signs_encounter.v1"
kind = "operational-template"
checksum = "sha256:..."
package = "anarchie-clinical-templates@1.0.0"
dependencies = [
  "openEHR-EHR-COMPOSITION.encounter.v1",
  "openEHR-EHR-OBSERVATION.blood_pressure.v2",
]
```

The lock records package version and digest, upstream revision and source path, artefact identifier and revision, lifecycle state, language inventory, hard dependency graph, slot constraints, licence evidence, attribution, generated artefact checksums, and compiler name/version/digest.

## Resolution Rules

Archetype identity and package versioning must not be conflated.

- `.v1`, `.v2`, and `.v3` in an archetype ID are compatibility lines. Resolution never silently crosses one of these major lines.
- The archetype metadata `revision` identifies a concrete revision within that line. The resolver may choose the highest policy-compatible published revision unless the manifest pins one; the lock always records the exact result.
- A specialisation parent is a hard dependency.
- An explicit archetype selected by a source template is a hard dependency.
- A selected slot filler is a hard dependency of the compiled template.
- A slot regex is retained as a constraint and does not cause every matching archetype to be installed.
- Missing or ambiguous hard dependencies fail resolution with a report showing the requesting artefact and candidate sources.
- An unavailable requested major version is an error. Replacing it with a newer major requires an explicit migration decision and new template release.
- Package releases have their own semantic or calendar version because CKM templates do not provide suitable package versions.

## Package Format

The first package format is a reproducible compressed archive containing data only:

```text
ckm-international-2026.8.1.tar.zst
|-- knowledge-package.toml
|-- artefacts/
|   |-- archetypes/
|   |-- templates/
|   |-- opts/
|   |-- web-templates/
|   |-- examples/
|   `-- conformance/
|-- provenance/
|   |-- attribution.json
|   |-- licences/
|   `-- source.json
`-- checksums.sha256
```

Packages contain no executable hooks. Installation enforces compressed and expanded size limits, file-count limits, safe relative paths, declared-file-only extraction, checksums, manifest validation, and atomic activation. Package content is installed by digest so multiple versions can coexist and a failed update leaves the previous lock active.

## Deployment Layout

```text
my-cdr/
|-- knowledge.toml
|-- knowledge.lock
|-- knowledge/
|   |-- packages/
|   |   `-- sha256-.../
|   |-- archetypes/
|   |-- templates/
|   |-- web-templates/
|   |-- examples/
|   |-- provenance/
|   `-- index.json
|-- ehrs/
`-- index/
```

The package store is authoritative for installed schema content. Human-friendly archetype/template indexes are derived and rebuildable. A template is stored by template ID and digest rather than overwritten at one path.

Every contribution that validates a Composition against a template records the template digest in contribution metadata. Updating the active template must never change how historical data is interpreted or whether it validates. Old locked template digests remain available while referenced.

## CLI Surface

Knowledge consumption remains part of the `anarchie` product:

```text
anarchie knowledge add <package>
anarchie knowledge remove <package>
anarchie knowledge resolve
anarchie knowledge install
anarchie knowledge update [package]
anarchie knowledge list
anarchie knowledge tree
anarchie knowledge why <artefact-id>
anarchie knowledge diff
anarchie knowledge audit
anarchie knowledge doctor
anarchie knowledge vendor
```

Artefact inspection is domain-oriented:

```text
anarchie archetype list
anarchie archetype show <archetype-id>
anarchie archetype dependencies <archetype-id>
anarchie archetype dependants <archetype-id>
anarchie template list
anarchie template show <template-id>
anarchie template example <template-id>
```

Package authoring is a separate command family but remains in the same binary while it shares the same parser, model, resolver, and verification code:

```text
anarchie pack build
anarchie pack inspect <archive>
anarchie pack verify <archive>
anarchie pack publish
```

The existing `anarchie pack add` and local-directory pack behaviour are the implementation seed. They should migrate toward the manifest/lock model rather than become a second package system.

## Package Build Pipeline

```text
pinned upstream commit
        |
        v
inventory source metadata and licences
        |
        v
construct hard-dependency and slot-constraint graphs
        |
        v
apply lifecycle, namespace, language, and licence policy
        |
        v
compile curated source templates with a pinned build-time compiler
        |
        v
import OPT, generate native OPT JSON and Web Template
        |
        v
generate examples and conformance mutations
        |
        v
validate with anarchie and independent oracles
        |
        v
emit reproducible package, lock data, attribution, and evidence report
```

The CKM mirror is ADL 1.4/OET while current Archie primarily targets ADL2/OPT2. The build pipeline therefore needs an explicit pinned legacy compiler path, such as a verified Archetype Designer/ADL Workbench/openEHR SDK process, until anarchie can ingest and compile those sources natively. Generated packages insulate users from that build-time toolchain.

## Distribution

The default packages are built during the anarchie release process, compressed, and embedded in the executable. This preserves offline first use and the single-binary promise. `anarchie init` expands the embedded package set; `--minimal` installs none; a future `--knowledge core` can select the smaller starter catalogue.

Independent package updates use immutable registry releases and checksums. Installation never reads from CKM `master` directly. GitHub Releases and a Git-backed index are sufficient for the first registry; a bespoke service is not required.

## Scope Boundary

The knowledge system is large, but it does not yet justify a separate end-user tool.

Keep one `anarchie` CLI and one shipped binary because:

- package state determines CDR validation behaviour;
- users need one transaction from package installation to template activation;
- the package manager reuses anarchie's AOM, OPT, validation, filesystem, licensing, and canonical serialisation code;
- offline installation is simplest when the default package is embedded;
- a separate executable would create a second configuration, release, error, and support surface before there is another consumer.

Maintain internal boundaries:

1. **Runtime data plane:** RM, validation, store, query, REST, and MCP. It only consumes resolved installed knowledge and must never invoke a JVM or network.
2. **Knowledge management plane:** manifest, resolver, lock, package store, activation, audit, and inspection. It may use the network only for explicit add/update operations.
3. **Package build plane:** upstream harvesting, ADL/OET compilation adapters, example generation, package publication, and provenance production. Heavy external compilers remain build-time tools.
4. **Conformance plane:** corpora and oracle adapters under `tests/` and `s/`. It produces evidence but is not linked into the runtime.

Extraction becomes justified only when evidence supplies one of these triggers:

- a second product needs the resolver/package format as a library;
- the package format and registry require an independent release cadence;
- build-time compiler dependencies materially damage normal binary size, build time, or portability;
- security requires package authoring/publishing to run outside the trusted CDR process;
- the module boundary cannot be maintained inside the crate without dependency cycles.

If extraction happens, `anarchie` remains the user-facing orchestrator and consumes a stable `anarchie-knowledge` library or package protocol. Repository extraction should preserve history and follow the same consumer-driven rule as the deferred RM library extraction. Size alone is not a reason to split a coherent product.

## Roadmap

Legend: `[x]` done, `[~]` partial, `[ ]` not started.

- [~] **K1 - Starter package seed.** Eight embedded templates, default installation, `--minimal`, local-directory packs, attribution, and template validation exist. They are not yet manifest/lock packages and several are hand-authored rather than reproducibly generated.
- [x] **K2 - CKM inventory.** `anarchie knowledge inventory <checkout>` deterministically inventories ADL 1.4/OET/termset sources, identifiers, revisions, lifecycle, languages, raw and normalised licence evidence, SHA-256 digests, local/remote provenance, specialisation and selected-archetype hard dependencies, symbolic slot constraints, duplicates, missing dependencies, major-version mismatches, and parse limitations from a pinned source checkout. The synthetic corpus and full International mirror snapshot both exercise it.
- [x] **K3 - Manifest and lock.** `anarchie init` creates a versioned `knowledge.toml`; deployment-aware `anarchie knowledge resolve <checkout>` produces deterministic `knowledge.lock`; `knowledge status` reports `unresolved`, `current`, or `stale` from manifest, source-revision, and inventory SHA-256 evidence; and `knowledge why` explains inclusion, exclusion, and dependency issues. Dirty sources, duplicates, ambiguity, policy-excluded dependencies, and major substitution are blocked, while failed resolution never overwrites a lock. The `knowledge-package-format-1` profile covers deterministic resolution, failures, provenance, and deployment freshness. Package-version and registry resolution, update diffs, and offline cache behaviour belong to K9 rather than this source-policy milestone.
- [ ] **K4 - Secure package format.** Implement reproducible archives, content-addressed storage, checksum verification, extraction limits, atomic install/rollback, and coexistence of package versions.
- [ ] **K5 - Published International package.** Build and embed the complete eligible published International archetype library with a closed hard-dependency graph and machine-readable inclusion/exclusion report.
- [ ] **K6 - Template compiler pipeline.** Pin the ADL 1.4/OET build path, import generated OPT XML, generate native OPT JSON and Web Templates, and prove byte-reproducible builds from one lock.
- [ ] **K7 - Executable clinical catalogue.** Expand templates into documented domain packages; require examples, invalid mutations, source closure, and conformance evidence for every active template.
- [ ] **K8 - Version-safe runtime integration.** Store template digests with contributions, retain historical templates, activate versions explicitly, block unsafe removal, and support exact lock rollback.
- [ ] **K9 - Registry and updates.** Publish immutable packages and a Git-backed registry index; implement explicit discovery/update with previews, caching, checksums, and optional attestations.
- [ ] **K10 - Release gate.** Require dependency closure, reproducibility, licence/attribution completeness, example validation, oracle comparison, and a package bill of materials before release.

## Completion Criteria

The batteries-included goal is complete when:

- a clean `anarchie init` installs the full supported knowledge base offline;
- every included archetype has identity, revision, lifecycle, source, digest, licence evidence, and dependency metadata;
- every hard dependency resolves without silent major substitution;
- every active template is reproducibly generated and has valid examples and independent conformance evidence;
- historical Compositions retain the exact template digest used at commit time;
- updating or rolling back `knowledge.lock` is deterministic and atomic;
- no terminology content is bundled;
- package and conformance reports state exclusions and limitations instead of hiding them.
