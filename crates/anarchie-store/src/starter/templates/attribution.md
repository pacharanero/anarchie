<!-- SPDX-License-Identifier: CC-BY-SA-3.0 -->
# Bundled starter templates - attribution & licence

The Operational Templates in this directory (`*.opt.json`) are **clinical data models, not `anarchie` source code**. They are *derivative works* (flattened Operational Templates) of openEHR International clinical archetypes published in the [openEHR Clinical Knowledge Manager](https://ckm.openehr.org/) under the `org.openehr` namespace.

These models are licensed under the **Creative Commons Attribution-ShareAlike 3.0** licence (CC-BY-SA 3.0, <https://creativecommons.org/licenses/by-sa/3.0/>), the same licence as the source archetypes. This is **independent of, and segregated from, the `anarchie` software licence** (AGPL-3.0-or-later): the two are distinct works distributed together. Per the ShareAlike condition, these templates and any further derivatives of them remain licensed CC-BY-SA 3.0. See [`specs/licensing.md`](../../../../../specs/licensing.md) for the full four-layer licensing position.

**Terminology disclaimer.** These templates contain terminology *bindings* (references to SNOMED CT / LOINC / openEHR codes) only - never terminology *content*. They do not include, and you must separately license, any clinical terminology (SNOMED CT, LOINC, ICD, etc.) if you want runtime terminology validation.

> ⚠️ Re-confirm the exact CC-BY-SA version and wording on the CKM Terms of Use page at packaging time; this file records the position as understood at authoring time and is not legal advice.

## Provenance per template

Every template's root concept is `openEHR-EHR-COMPOSITION.encounter.v1` (custodian: openEHR Foundation; © openEHR Foundation; CC-BY-SA 3.0). All source archetypes below are custodianed by the **openEHR Foundation** (`org.openehr`) and licensed **CC-BY-SA 3.0**.

### `vital_signs_encounter.v1`

Derived from the openEHR International vital-signs OBSERVATION archetypes (each © openEHR Foundation):

| Source archetype | Concept |
|---|---|
| `openEHR-EHR-OBSERVATION.blood_pressure.v2` | Blood pressure |
| `openEHR-EHR-OBSERVATION.pulse.v1` | Pulse/heart beat |
| `openEHR-EHR-OBSERVATION.body_temperature.v2` | Body temperature |
| `openEHR-EHR-OBSERVATION.respiration.v1` | Respiration |
| `openEHR-EHR-OBSERVATION.body_weight.v2` | Body weight |
| `openEHR-EHR-OBSERVATION.height.v2` | Height/length |

### `problem_list.v1`

| Source archetype | Concept | Copyright |
|---|---|---|
| `openEHR-EHR-EVALUATION.problem_diagnosis.v1` | Problem/Diagnosis | © openEHR Foundation |

### `adverse_reaction_list.v1`

| Source archetype | Concept | Copyright |
|---|---|---|
| `openEHR-EHR-EVALUATION.adverse_reaction_risk.v1` | Adverse reaction risk | © NEHTA, openEHR Foundation, HL7 International, Nasjonal IKT |
