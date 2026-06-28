# IPS Readiness — the gap to a full International Patient Summary demo

This document chases down the gap between what `anarchie` does today and a convincing end-to-end demonstration of the **International Patient Summary (IPS)**. It is the practical follow-on to [bundled-archetypes.md](bundled-archetypes.md) (which set IPS as the organising target for the starter bundle) and [regulatory-context.md](regulatory-context.md) (which framed the "openEHR behind, FHIR in front" pattern that an IPS demo must exercise).

> **What "a full IPS demo" means.** The IPS (HL7 FHIR International Patient Summary; CEN/ISO 17269) is a *minimal, non-exhaustive, condition-independent* summary, defined as a **FHIR document** - a `Bundle` containing a `Composition` resource whose sections reference `Condition`, `AllergyIntolerance`, `MedicationStatement`, and friends. openEHR has no native "IPS" artefact. So a full demo is necessarily two-sided: **store** the clinical content as openEHR Compositions, then **project** it to a conformant FHIR IPS Bundle. Both sides have gaps today.

---

## The three layers of the gap

A working IPS demo needs all three, in order:

1. **Content templates** - openEHR Operational Templates to author and validate each IPS section's content. *Complete: all 8 Tier-1 templates now ship in `ips-core`.*
2. **A FHIR IPS projection** - a derived consumer that turns Compositions into a FHIR IPS Bundle. *Not built (roadmap Phase 6).*
3. **Terminology** - the SNOMED CT / LOINC / etc. codes and value sets the IPS profiles require. *Deliberately not bundled; binding validation deferred to `sct`.*

The good news: none of this needs new Reference Model work. The RM types every IPS section relies on - `OBSERVATION`, `EVALUATION`, `INSTRUCTION`, `ACTION`, `ACTIVITY`, `ISM_TRANSITION`, the `DV_*` values - are all already implemented and round-tripping (see [reference-model-coverage.md](reference-model-coverage.md)). The gap is **content models and a projection**, not substrate.

---

## Layer 1 — content templates (8 of 8, complete)

The IPS section set, each mapped to the openEHR archetype that carries it and to the bundled template that should exist. IPS conformance tiers: **R** = required, **r** = recommended, **o** = optional.

| IPS section | Tier | openEHR archetype(s) | Template | Status |
|---|---|---|---|---|
| Problem List | R | `EVALUATION.problem_diagnosis` | `problem_list.v1` | ✅ shipped |
| Allergies & Intolerances | R | `EVALUATION.adverse_reaction_risk` | `adverse_reaction_list.v1` | ✅ shipped |
| Medication Summary | **R** | `OBSERVATION.medication_statement.v0` (+ `INSTRUCTION.medication_order.v3`) | `medication_list.v1` | ✅ shipped (v0 source) |
| Vital Signs | r | `OBSERVATION.blood_pressure`, `.pulse`, `.body_temperature`, `.body_weight`, `.height`, … | `vital_signs_encounter.v1` | ✅ shipped |
| Results (laboratory) | r | `OBSERVATION.laboratory_test_result.v1` (+ `CLUSTER.laboratory_test_analyte`, `CLUSTER.specimen`) | `laboratory_result_report.v1` | ✅ shipped |
| History of Procedures | r | `ACTION.procedure.v1` | `procedure_list.v1` | ✅ shipped |
| Immunizations | r | `ACTION.medication.v1` (vaccine) | `immunisation_list.v1` | ✅ shipped |
| Encounter scaffold | n/a | `COMPOSITION.encounter` (+ `EVALUATION.clinical_synopsis.v1`) | `encounter_note.v1` | ✅ shipped |
| Medical Devices | r | `EVALUATION.device_summary` / `CLUSTER.device` | (none) | ❌ not in Tier 1 |

**Layer 1 is complete: all eight Tier-1 templates now ship in `ips-core`.** That spans all three *required* IPS sections (Problems, Allergies, Medications) plus the recommended Vital Signs, Results, Procedures, and Immunizations, and an encounter-note scaffold. Each was authored against the real CKM archetype at-codes and is covered by a validation regression test (a sample Composition validates against the template). The only Tier-1 section still unbundled is **Medical Devices** (a recommended section), deliberately deferred.

### What authoring a template costs

The bundled templates are anarchie's own flattened OPT JSON - a tree of `COMPLEX` nodes (`rm_type` / `node_id` / `occurrences` / `attributes`) with leaf constraints (`C_STRING`, `CODE_PHRASE`, `C_DV_QUANTITY`, `C_DV_ORDINAL`, …). They are deliberately **minimal** for the MVP: `problem_list.v1`, for instance, constrains only the problem-name `ELEMENT`. A medication or procedure list at the same fidelity is a similarly small, single-`ENTRY` tree - a few hours each, hand-authored against the archetype's real at-codes.

Two honest caveats, both inherited from [roadmap.md](roadmap.md):

- **At-codes must be correct against the real CKM archetype**, or a Composition authored to the template will not interoperate with other openEHR systems. The eight bundled templates were hand-authored against the real at-codes (read straight from the published ADL); the durable path for re-generating them is to flatten the curated `.oet` template with Archetype Designer / ADL Workbench / Archie and ingest the resulting `.opt` XML (the open Phase 3 item).
- **Medications and immunisations are an "action"-flavoured model, not a simple list.** The summary-friendly archetype is `OBSERVATION.medication_statement.v0` (it maps cleanly to FHIR `MedicationStatement`); the fuller medication lifecycle (`INSTRUCTION.medication_order.v3` + `ACTION.medication` with `ISM_TRANSITION`) is richer than the problem/allergy list pattern and maps to FHIR `MedicationRequest`. For an IPS *summary*, `medication_statement` is the right target - note it is still at **v0 (draft)** in the CKM, so the bundled template should be re-flattened when it stabilises. `medication_list.v1` was built this way (medication name required; route and clinical indication optional).

All eight Tier-1 templates are built. The Layer-1 follow-ups are now optional polish: landing `.opt` XML ingest to re-generate them from CKM artefacts, adding the Medical Devices section, and enriching the minimal templates (e.g. the laboratory analyte/result-value cluster) beyond their name-and-summary fields.

---

## Layer 2 — the openEHR → FHIR IPS projection (not built)

This is the larger, more interesting gap, and the piece that makes it an *IPS* demo rather than an openEHR demo. Per [regulatory-context.md](regulatory-context.md) it slots cleanly into the onion model as one more derived consumer over the canonical store, alongside the AQL index, the REST API, and the MCP server - regenerable, never authoritative.

A minimal one-way exporter is enough to demonstrate IPS. The mapping is direct:

| openEHR source | FHIR IPS resource |
|---|---|
| `EVALUATION.problem_diagnosis` | `Condition` (IPS Condition profile) |
| `EVALUATION.adverse_reaction_risk` | `AllergyIntolerance` |
| `OBSERVATION.medication_statement.v0` | `MedicationStatement` (+ `Medication`) |
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
2. Each section stored as a validated openEHR Composition. All four templates this needs (`problem_list.v1`, `adverse_reaction_list.v1`, `medication_list.v1`, `vital_signs_encounter.v1`) now ship in `ips-core`.
3. `anarchie export-ips <ehr>` producing a FHIR IPS `Bundle` for that patient.
4. That Bundle passing the HL7 FHIR IPS validator at build time.

Results, procedures, and immunisations now also ship as templates, so they can join the demo for a richer record; medical devices and the optional sections remain beyond this minimal milestone.

---

## Recommended sequence

1. ✅ **Tier-1 templates done** - all eight `ips-core` templates are built and validated end to end against sample Compositions, covering every required IPS section (and most recommended ones).
2. **Build the `anarchie-fhir` projection** and `anarchie export-ips`, validated against the IPS profiles with the HL7 FHIR validator at test time. This is now the critical-path gap to a *demonstrable* IPS.
3. **Create the synthetic demo records** in a separate, reusable content repo (see below) and wire a load script (`anarchie init` → `commit` loop).
4. **Optional Layer-1 polish:** land `.opt` XML ingest to re-generate the templates from CKM artefacts, add the Medical Devices section, and enrich the minimal templates (e.g. lab result-value clusters, structured dose/timing).

> **Note on `ips-core` today.** `anarchie pack add ips-core` now installs the full 8-template Tier-1 IPS span - all three required sections plus the recommended Vital Signs, Results, Procedures, and Immunizations. The remaining gap to a *demonstrable* IPS is the FHIR projection (Layer 2), not the content models.

---

## Demo medical-record content (separate repo)

The synthetic patient content is deliberately *not* part of this repo: it is needed across several sibling projects (`anarchie`, [`gitehr`](https://gitehr.org/), [`sct`](https://github.com/pacharanero/sct), `kam`) and in more than one format. A dedicated content repo should hold:

- **A handful of synthetic personas** with coherent clinical stories (e.g. a multimorbid older adult, a paediatric case, a pregnancy), explicitly synthetic and carrying no real PII - ideally aligned to the published IPS example patients so the openEHR and FHIR sides line up.
- **openEHR canonical-JSON Compositions** per patient that validate against the `ips-core` templates, plus a loader script that builds a CDR from them.
- **Parallel FHIR IPS Bundles** for the same patients (so the repo serves FHIR consumers directly, and gives the projection in Layer 2 a reference target to diff against).
- **A permissive data licence** (CC0 or CC-BY), kept distinct from any code.

This repo is the substrate every IPS demo draws on; it is tracked as its own piece of work rather than folded into `anarchie`.
