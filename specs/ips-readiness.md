# IPS Readiness — the gap to a full International Patient Summary demo

This document chases down the gap between what `anarchie` does today and a convincing end-to-end demonstration of the **International Patient Summary (IPS)**. It is the practical follow-on to [bundled-archetypes.md](bundled-archetypes.md) (which set IPS as the organising target for the starter bundle) and [regulatory-context.md](regulatory-context.md) (which framed the "openEHR behind, FHIR in front" pattern that an IPS demo must exercise).

> **What "a full IPS demo" means.** The IPS (HL7 FHIR International Patient Summary; CEN/ISO 17269) is a *minimal, non-exhaustive, condition-independent* summary, defined as a **FHIR document** - a `Bundle` containing a `Composition` resource whose sections reference `Condition`, `AllergyIntolerance`, `MedicationStatement`, and friends. openEHR has no native "IPS" artefact. So a full demo is necessarily two-sided: **store** the clinical content as openEHR Compositions, then **project** it to a conformant FHIR IPS Bundle. Both sides have gaps today.

---

## The three layers of the gap

A working IPS demo needs all three, in order:

1. **Content templates** - openEHR Operational Templates to author and validate each IPS section's content. *Partially present: 3 of the 8 Tier-1 templates.*
2. **A FHIR IPS projection** - a derived consumer that turns Compositions into a FHIR IPS Bundle. *Not built (roadmap Phase 6).*
3. **Terminology** - the SNOMED CT / LOINC / etc. codes and value sets the IPS profiles require. *Deliberately not bundled; binding validation deferred to `sct`.*

The good news: none of this needs new Reference Model work. The RM types every IPS section relies on - `OBSERVATION`, `EVALUATION`, `INSTRUCTION`, `ACTION`, `ACTIVITY`, `ISM_TRANSITION`, the `DV_*` values - are all already implemented and round-tripping (see [reference-model-coverage.md](reference-model-coverage.md)). The gap is **content models and a projection**, not substrate.

---

## Layer 1 — content templates (3 of 8)

The IPS section set, each mapped to the openEHR archetype that carries it and to the bundled template that should exist. IPS conformance tiers: **R** = required, **r** = recommended, **o** = optional.

| IPS section | Tier | openEHR archetype(s) | Template | Status |
|---|---|---|---|---|
| Problem List | R | `EVALUATION.problem_diagnosis` | `problem_list.v1` | ✅ shipped |
| Allergies & Intolerances | R | `EVALUATION.adverse_reaction_risk` | `adverse_reaction_list.v1` | ✅ shipped |
| Medication Summary | **R** | `EVALUATION.medication_statement` (+ `INSTRUCTION.medication_order`) | `medication_list.v1` | ❌ **missing** |
| Vital Signs | r | `OBSERVATION.blood_pressure`, `.pulse`, `.body_temperature`, `.body_weight`, `.height`, … | `vital_signs_encounter.v1` | ✅ shipped |
| Results (laboratory) | r | `OBSERVATION.laboratory_test_result` (+ `CLUSTER.laboratory_test_analyte`, `CLUSTER.specimen`) | `laboratory_result_report.v1` | ❌ **missing** |
| History of Procedures | r | `ACTION.procedure` | `procedure_list.v1` | ❌ **missing** |
| Immunizations | r | `ACTION.medication` (vaccine) | `immunisation_list.v1` | ❌ **missing** |
| Medical Devices | r | `EVALUATION.device_summary` / `CLUSTER.device` | (none) | ❌ not in Tier 1 |
| Encounter scaffold | n/a | `COMPOSITION.encounter` (+ `EVALUATION.clinical_synopsis`) | `encounter_note.v1` | ❌ **missing** |

**The headline gap is the Medication Summary** - it is one of the three *required* IPS sections and is the only required section with no template today. Results, Procedures, and Immunizations are *recommended* and are what make a summary look clinically real in a demo.

### What authoring a template costs

The bundled templates are anarchie's own flattened OPT JSON - a tree of `COMPLEX` nodes (`rm_type` / `node_id` / `occurrences` / `attributes`) with leaf constraints (`C_STRING`, `CODE_PHRASE`, `C_DV_QUANTITY`, `C_DV_ORDINAL`, …). They are deliberately **minimal** for the MVP: `problem_list.v1`, for instance, constrains only the problem-name `ELEMENT`. A medication or procedure list at the same fidelity is a similarly small, single-`ENTRY` tree - a few hours each, hand-authored against the archetype's real at-codes.

Two honest caveats, both inherited from [roadmap.md](roadmap.md):

- **At-codes must be correct against the real CKM archetype**, or a Composition authored to the template will not interoperate with other openEHR systems. The MVP authors these by hand; the durable path is to flatten the curated `.oet` template with Archetype Designer / ADL Workbench / Archie and ingest the resulting `.opt` XML (the open Phase 3 item). Building the missing five is the moment to decide whether to land `.opt` XML ingest first.
- **Medications and immunisations are an "action" model, not a simple list.** `medication_statement` is the summary-friendly `EVALUATION`; the fuller medication lifecycle (`INSTRUCTION.medication_order` + `ACTION.medication` with `ISM_TRANSITION`) is richer than the problem/allergy list pattern. For an IPS *summary*, `EVALUATION.medication_statement` is the right first target.

**Priority order:** `medication_list` (required) → `laboratory_result_report` → `immunisation_list` → `procedure_list` → `encounter_note`.

---

## Layer 2 — the openEHR → FHIR IPS projection (not built)

This is the larger, more interesting gap, and the piece that makes it an *IPS* demo rather than an openEHR demo. Per [regulatory-context.md](regulatory-context.md) it slots cleanly into the onion model as one more derived consumer over the canonical store, alongside the AQL index, the REST API, and the MCP server - regenerable, never authoritative.

A minimal one-way exporter is enough to demonstrate IPS. The mapping is direct:

| openEHR source | FHIR IPS resource |
|---|---|
| `EVALUATION.problem_diagnosis` | `Condition` (IPS Condition profile) |
| `EVALUATION.adverse_reaction_risk` | `AllergyIntolerance` |
| `EVALUATION.medication_statement` | `MedicationStatement` (+ `Medication`) |
| `OBSERVATION.laboratory_test_result` | `Observation` (laboratory) + `DiagnosticReport` |
| `OBSERVATION.blood_pressure` etc. | `Observation` (vital signs) |
| `ACTION.procedure` | `Procedure` |
| `ACTION.medication` (vaccine) | `Immunization` |
| the patient / EHR | `Patient` |
| the assembled summary | `Composition` + `Bundle` (document) |

Design notes:

- **Shape:** a new `anarchie-fhir` crate and an `anarchie export-ips <ehr>` command (and a future `GET …/ips` REST endpoint) that walks an EHR's head Compositions, maps each ENTRY to its FHIR resource, and assembles a `Bundle` of type `document` with an IPS `Composition` and the mandated sections.
- **Scope discipline:** this is a *convenience projection, not a certified EHDS/EEHRxF gateway* - exactly the line drawn in [regulatory-context.md](regulatory-context.md). One-way (openEHR → FHIR) is enough for a demo; round-tripping is explicitly out of scope.
- **Validation of the output:** the demo is far more convincing if the emitted Bundle validates against the IPS profiles with the HL7 FHIR validator (a build/test-time oracle, like Archie for openEHR validation - never a runtime dependency).

This is genuinely new code, but it is self-contained and additive: it reads canonical JSON and emits FHIR JSON, touching nothing in the store.

---

## Layer 3 — terminology (out of scope, by design)

The IPS profiles bind to SNOMED CT, LOINC, UCUM, and specific value sets. `anarchie` deliberately ships archetype *bindings* but no terminology *content* ([bundled-archetypes.md](bundled-archetypes.md), [licensing.md](licensing.md)). For a demo this is fine: the sample content carries plausible, hand-chosen codes (and UCUM units, which are free), and binding *validation* stays a later, optional step delegated to [`sct`](https://github.com/pacharanero/sct) via FHIR `$validate-code` (roadmap Phase 6). A demo should state plainly that codes are illustrative and not terminology-validated.

---

## The minimal viable IPS demo

The smallest thing that demonstrates IPS end to end, and a good milestone definition:

1. A synthetic patient whose record contains the **three required sections** - problems, allergies, medications - plus **vital signs** for colour.
2. Each section stored as a validated openEHR Composition (so it needs `medication_list.v1` - the one required-section template still missing).
3. `anarchie export-ips <ehr>` producing a FHIR IPS `Bundle` for that patient.
4. That Bundle passing the HL7 FHIR IPS validator at build time.

Everything beyond that - results, procedures, immunisations, medical devices, the optional sections - widens coverage but is not needed to *prove the pattern*.

---

## Recommended sequence

1. **Decide the authoring path** for templates: keep hand-authoring minimal OPT JSON for the MVP, or land `.opt` XML ingest (Phase 3 open item) first so the five new templates come from real flattened CKM artefacts. Hand-authoring is faster to a demo; XML ingest is the durable answer.
2. **Author `medication_list.v1`** (required section) and validate a sample medication-statement Composition against it end to end - the same proof already done for vitals.
3. **Add `laboratory_result_report.v1`, `immunisation_list.v1`, `procedure_list.v1`, `encounter_note.v1`** to complete the Tier-1 IPS span, growing the `ips-core` pack from 3 templates to 8.
4. **Build the `anarchie-fhir` projection** and `anarchie export-ips`, validated against the IPS profiles at test time.
5. **Create the synthetic demo records** in a separate, reusable content repo (see below) and wire a load script (`anarchie init` → `commit` loop).

> **Note on `ips-core` today.** `anarchie pack add ips-core` currently installs the 3-template starter set, not the full 8-section IPS span. Closing Layer 1 is precisely what makes the pack name honest.

---

## Demo medical-record content (separate repo)

The synthetic patient content is deliberately *not* part of this repo: it is needed across several sibling projects (`anarchie`, [`gitehr`](https://gitehr.org/), [`sct`](https://github.com/pacharanero/sct), `kam`) and in more than one format. A dedicated content repo should hold:

- **A handful of synthetic personas** with coherent clinical stories (e.g. a multimorbid older adult, a paediatric case, a pregnancy), explicitly synthetic and carrying no real PII - ideally aligned to the published IPS example patients so the openEHR and FHIR sides line up.
- **openEHR canonical-JSON Compositions** per patient that validate against the `ips-core` templates, plus a loader script that builds a CDR from them.
- **Parallel FHIR IPS Bundles** for the same patients (so the repo serves FHIR consumers directly, and gives the projection in Layer 2 a reference target to diff against).
- **A permissive data licence** (CC0 or CC-BY), kept distinct from any code.

This repo is the substrate every IPS demo draws on; it is tracked as its own piece of work rather than folded into `anarchie`.
