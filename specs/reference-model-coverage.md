# Reference Model Coverage

The openEHR Reference Model has roughly 120 types across several packages. `anarchie` does not need all of them at once - a CDR that stores and validates Compositions needs the Composition, Data Structures, Data Types, and core identification types first, and can defer the demographics-adjacent and change-control machinery that git already subsumes.

This document is the running coverage tracker: which RM types are implemented in `anarchie-rm` today, and which are deferred and why. It is the practical answer to the Phase 1 learning question ("can we represent real Compositions losslessly?") and the place to look before assuming a type exists.

Legend: **done** = implemented with canonical-JSON round-trip; **partial** = present with a known gap; **deferred** = intentionally not yet modelled.

---

## Identification (BASE)

| Type | Status | Notes |
|---|---|---|
| `OBJECT_ID` (abstract) | done | Modelled as the `ObjectId` enum |
| `UID_BASED_ID` (abstract) | done | `UidBasedId` enum |
| `HIER_OBJECT_ID` | done | |
| `OBJECT_VERSION_ID` | deferred | `version_uid` is currently a formatted string (`object_id::system_id::version_tree_id`); a parsed type is future work |
| `ARCHETYPE_ID` | done | |
| `TERMINOLOGY_ID` | done | |
| `GENERIC_ID` | deferred | |
| `OBJECT_REF` | deferred | Referenced indirectly; not a first-class type yet |
| `LOCATABLE_REF` | done | |
| `PARTY_REF` | done | |
| `UID` (UUID / INTERNET_ID / ISO_OID) | deferred | UUIDs handled as strings |
| `VERSION_TREE_ID` | deferred | Carried inside the `version_uid` string |

## Data Types (RM Data Types)

| Type | Status | Notes |
|---|---|---|
| `DV_TEXT` | done | |
| `DV_CODED_TEXT` | done | |
| `DV_PARAGRAPH` | deferred | |
| `DV_BOOLEAN` | done | |
| `DV_IDENTIFIER` | partial | Standalone `DV_IDENTIFIER` inside `PARTY_IDENTIFIED.identifiers` does not yet re-emit its `_type` (the one known Phase 1 gap) |
| `DV_STATE` | deferred | |
| `DV_ORDINAL` | done | |
| `DV_SCALE` | done | |
| `DV_QUANTITY` | done | |
| `DV_COUNT` | done | |
| `DV_PROPORTION` | done | |
| `DV_DURATION` | done | |
| `DV_DATE` / `DV_TIME` / `DV_DATE_TIME` | done | |
| `DV_PARSABLE` | done | |
| `DV_MULTIMEDIA` | done | |
| `DV_URI` / `DV_EHR_URI` | done | |
| `DV_INTERVAL<T>` | deferred | Constraint-side `Interval<T>` exists in `anarchie-aom`; the data-value `DV_INTERVAL` is not yet modelled |
| `CODE_PHRASE` | done | |
| `TERM_MAPPING` | done | |
| `REFERENCE_RANGE<T>` | deferred | |
| Abstract ordered/quantified ancestors | n/a | Rust has no inheritance; shared fields are inlined per concrete type |

## Data Structures (RM Data Structures)

| Type | Status | Notes |
|---|---|---|
| `ITEM` (abstract) | done | `Item` enum |
| `ELEMENT` | done | |
| `CLUSTER` | done | |
| `ITEM_SINGLE` / `ITEM_LIST` / `ITEM_TABLE` / `ITEM_TREE` | done | `ItemStructure` enum |
| `HISTORY<T>` | done | |
| `EVENT<T>` (abstract) | done | `Event` enum |
| `POINT_EVENT` / `INTERVAL_EVENT` | done | |

## Composition (RM Composition / EHR)

| Type | Status | Notes |
|---|---|---|
| `COMPOSITION` | done | |
| `SECTION` | done | |
| `OBSERVATION` / `EVALUATION` / `INSTRUCTION` / `ACTION` / `ADMIN_ENTRY` | done | `ContentItem` / entry types |
| `ACTIVITY` | done | |
| `EVENT_CONTEXT` | done | |
| `ISM_TRANSITION` | done | |
| `INSTRUCTION_DETAILS` | done | |
| `CARE_ENTRY` / `ENTRY` (abstract) | n/a | Shared fields inlined per concrete entry |

## Common / parties / audit

| Type | Status | Notes |
|---|---|---|
| `PARTY_PROXY` (abstract) | done | `PartyProxy` enum |
| `PARTY_SELF` / `PARTY_IDENTIFIED` / `PARTY_RELATED` | done | |
| `PARTICIPATION` | done | |
| `ARCHETYPED` | done | |
| `LINK` | done | |
| `AUDIT_DETAILS` | partial | Audit is currently carried by the git commit + contribution manifest rather than a standalone RM struct; a typed `AUDIT_DETAILS` is future work |
| `ATTESTATION` | deferred | |
| `FEEDER_AUDIT` / `FEEDER_AUDIT_DETAILS` | deferred | |

## EHR / change control

| Type | Status | Notes |
|---|---|---|
| `EHR` | done | |
| `EHR_STATUS` | done | |
| `EHR_ACCESS` | deferred | Access control is out of MVP scope ([rest-api.md](rest-api.md)) |
| `FOLDER` / directory | deferred | On-disk `folders/` layout reserved ([on-disk-format.md](on-disk-format.md)); the RM type is not yet modelled |
| `VERSIONED_OBJECT<T>` / `VERSIONED_COMPOSITION` | deferred | The version set is represented by a git history + a compositions directory, not a materialised RM object |
| `VERSION<T>` / `ORIGINAL_VERSION` / `IMPORTED_VERSION` | deferred | Versioning is intrinsic to git ([versioning-and-git.md](versioning-and-git.md)); a typed `ORIGINAL_VERSION` wrapper is future work, notably for the REST `versioned_composition` endpoints |
| `CONTRIBUTION` | partial | Persisted as a manifest file + a git commit; not yet a first-class RM struct |

---

## Why some types are deferred, not missing

`anarchie` deliberately lets **git subsume the change-control package**. `VERSIONED_OBJECT`, `VERSION`, `ORIGINAL_VERSION`, and `CONTRIBUTION` describe exactly what a git history already encodes - an audited chain of immutable versions grouped into atomic commits - so they are represented by the commit graph plus lightweight manifests rather than materialised RM objects (see [versioning-and-git.md](versioning-and-git.md)). They graduate from *deferred* to *done* only where a REST endpoint must hand a client a literal `ORIGINAL_VERSION<COMPOSITION>` or `CONTRIBUTION` object, at which point the type is synthesised on read from git + manifest. The demographics-adjacent types (`EHR_ACCESS`, full `FEEDER_AUDIT`) are out of MVP scope per [architecture.md](architecture.md).
