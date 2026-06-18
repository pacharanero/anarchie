# openEHR Terminology Codes (reference)

A handful of openEHR-internal code groups (terminology id `openehr`) appear throughout the Reference Model as the values of `DV_CODED_TEXT` attributes - change types, lifecycle states, composition category, the instruction state machine, and so on. The validator and the audit/commit path need these as a small built-in table, since they are fixed by the RM rather than supplied by an archetype.

This document collects the groups `anarchie` cares about as a convenient reference. **It is not normative.** The authoritative source is the openEHR Terminology, published at [terminology.openehr.org](https://terminology.openehr.org) and mirrored in the `openEHR/terminology.openehr.org` repository in this workspace. Re-confirm any code against that source before relying on it; the tables below are a working aid, not a substitute.

---

## Change type (`AUDIT_DETAILS.change_type`)

The kind of change a `CONTRIBUTION`/version records. `anarchie` maps these onto the `anarchie-change-type` git trailer.

| Code | Value | Meaning |
|---|---|---|
| 249 | creation | First version of an object |
| 250 | amendment | Correction that does not change clinical meaning |
| 251 | modification | A clinically meaningful change |
| 252 | synthesis | Derived/aggregated from other versions |
| 523 | deleted | Logical deletion (a new "deleted" version) |

## Version lifecycle state (`ORIGINAL_VERSION.lifecycle_state`)

| Code | Value | Meaning |
|---|---|---|
| 532 | complete | The version is finished |
| 553 | incomplete | A draft, not yet complete |
| 523 | deleted | The version is logically deleted |

## Composition category (`COMPOSITION.category`)

| Code | Value | Meaning |
|---|---|---|
| 431 | persistent | Long-lived, continuously updated (problem list, medication list) |
| 433 | event | A point-in-time clinical event (an encounter) |
| 451 | episodic | Scoped to a care episode |

## Null flavour (`ELEMENT.null_flavour`)

Used when an `ELEMENT` carries no `value`. The commonly-used members of the openEHR null-flavour group:

| Code | Value | Meaning |
|---|---|---|
| 253 | unknown | Value is unknown |
| 271 | no information | No information is available |
| 272 | masked | Value withheld (for example for confidentiality) |
| 273 | not applicable | The element does not apply in this context |

## Instruction state machine (ISM) states

The careflow states an `ACTION.ism_transition.current_state` moves through (see [the ISM in the RM](https://specifications.openehr.org/releases/RM/latest/ehr.html)).

| Code | State | Meaning |
|---|---|---|
| 524 | initial | Not yet started |
| 526 | planned | Planned to be carried out |
| 527 | postponed | Planned but postponed |
| 528 | cancelled | Cancelled before starting |
| 529 | scheduled | Scheduled for a specific time |
| 245 | active | Currently being carried out |
| 530 | suspended | Temporarily halted |
| 531 | aborted | Abandoned before completion |
| 532 | completed | Successfully completed |

---

## Non-coded enumerations baked into the RM

A few RM attributes are small fixed enumerations rather than `openehr`-terminology codes. They live in the Rust types directly.

**`DV_PROPORTION.type`** (`ProportionKind`):

| Value | Kind |
|---|---|
| 0 | ratio |
| 1 | unitary |
| 2 | percent |
| 3 | fraction |
| 4 | integer fraction |

**`DV_QUANTIFIED.magnitude_status`** - a string drawn from the set `=`, `<`, `>`, `<=`, `>=`, `~` (where present; absent means an exact value). The validator enforces membership of this set.

---

## How `anarchie` uses these

- **Validation** ([validation.md](validation.md)) checks RM-level invariants against these tables: a `magnitude_status` outside the permitted set, or a `DV_PROPORTION.type` outside `0..=4`, is an error.
- **Commit/audit** ([versioning-and-git.md](versioning-and-git.md)) records `change_type` as a git trailer and in the contribution manifest.
- **Terminology validation of external bindings** (SNOMED CT, LOINC) is a separate, delegated concern handled by an optional backend such as [`sct`](https://github.com/pacharanero/sct) - see [validation.md](validation.md). Only the `openehr`-internal groups above are built in.
