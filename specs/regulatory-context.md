# Regulatory Context — EHDS, EEHRxF, xDHR, ESHIA

This document situates `anarchie` within the European regulatory and interoperability landscape. The purpose is to be **honest about fit**: where `anarchie` aligns with these frameworks, where it can usefully contribute, and - importantly - where it deliberately does *not* claim conformance. `anarchie` is a learning/experimental openEHR CDR, not a certified EHR system for the EU market.

> **Verification note.** EHDS, EEHRxF and ESHIA are confirmed from primary sources (the European Commission EHDS pages and ESHIA's own founding materials). The precise scope of **xDHR** is stated here with moderate confidence as HL7 Europe's "Exchange of Digital Health Records" implementation-guide family; confirm against HL7 Europe sources before relying on the detail.

---

## The four layers: from law to code

These four acronyms are often used interchangeably, but they sit at different levels of abstraction. Keeping them distinct is the whole point of this document.

```
EHDS    (Regulation, in force Mar 2025)      ← the LAW: mandates interoperability + EHR-system certification
   │ requires
EEHRxF  (European EHR exchange Format)       ← the FORMAT: what data must look like to cross borders
   │ operationalised by
xDHR + HL7 Europe / CEN implementation guides ← the SPECS: concrete FHIR IGs per data category
   │ adoption supported by
ESHIA   (AISBL, Brussels, Sept 2025)         ← the ALLIANCE: the EEHRxF "Standards & Policy Hub"
```

### EHDS — European Health Data Space (the law)

- A binding EU **Regulation**, in force since **26 March 2025**, with obligations phasing in over several years.
- **Primary use** (care delivery): patients can access, control, and share their health data across borders. Priority categories go live in stages:
  - **March 2029** — Patient Summaries and ePrescriptions/eDispensations.
  - **March 2031** — medical images, laboratory results, and hospital discharge reports.
- **Secondary use** (research, policy, innovation): a separate permit-based regime via Health Data Access Bodies.
- Introduces **mandatory certification of EHR systems** placed on the EU market against interoperability and security criteria. *This is the part `anarchie` must be careful about — see "What anarchie does not claim".*

### EEHRxF — European Electronic Health Record exchange Format (the format)

- The harmonised exchange format the Regulation points to (building on the earlier Commission Recommendation on a European EHR exchange format).
- Defines the *content and structure* that health data must take to be exchanged across EHDS - the priority categories above are each given an EEHRxF shape.

### xDHR and the implementation guides (the specs)

- **xDHR** — understood as HL7 Europe's **"Exchange of Digital Health Records"** project: concrete **FHIR implementation guides** that operationalise EEHRxF for the priority data categories.
- Alongside CEN/TC 251 standards work, these IGs are what an implementer actually builds against - the on-the-wire FHIR resources and profiles.

### ESHIA — European Standards for Health Interoperability Alliance (the alliance)

- A non-profit **AISBL** registered in Belgium on **18 September 2025**, founded by eighteen partners including CEN/TC 251, HL7 Europe, IHE-Europe, IEEE, MedCom, and several national competence centres.
- **Not a standard.** ESHIA hosts the **EEHRxF Standards & Policy Hub** and exists to help organisations move "from compliance to integration to strategic value" with EEHRxF.
- Relevance to `anarchie`: ESHIA is the convening/adoption body and a signal of where the ecosystem's energy is going, rather than something `anarchie` conforms *to*.

---

## openEHR vs FHIR/EEHRxF: complementary, not competing

The single most important framing: **EHDS/EEHRxF is a FHIR-shaped *exchange* world; openEHR is a *persistence and modelling* world.** They operate at different points in the stack and are increasingly used together:

| Concern | openEHR (anarchie's world) | EEHRxF / xDHR (EHDS's world) |
|---|---|---|
| Primary role | System of record; rich clinical modelling | Cross-border *exchange* of defined datasets |
| Granularity | Maximal dataset (archetypes capture everything) | Use-case datasets (Patient Summary, etc.) |
| Technology | RM + archetypes + AQL | FHIR resources + profiles + IGs |
| Lifecycle | Long-term storage, versioning, query | Document/message produced at exchange time |

The mainstream architecture pattern is **openEHR behind, FHIR in front**: an openEHR CDR is the durable store, and FHIR/EEHRxF artefacts are *projected* from it at the exchange boundary. `anarchie` fits exactly this pattern.

---

## Where anarchie fits

Three genuine points of alignment, plus the standard caution.

### 1. Content overlap is near-exact (and already specced)

The EHDS priority categories - Patient Summary, problems, medications, allergies, laboratory results, discharge reports - are *precisely* the IPS-aligned starter bundle already designed in [bundled-archetypes.md](bundled-archetypes.md). This is not a coincidence: the **International Patient Summary (IPS)** is the clinical backbone of the EEHRxF Patient Summary. So `anarchie`'s "batteries included" set is, by construction, aimed at the same clinical content EHDS prioritises. This makes `anarchie` a natural sandbox for *modelling and storing* EEHRxF-relevant content, even though it is not an exchange endpoint.

### 2. An openEHR → EEHRxF/FHIR projection is a natural consumer layer

In the onion model ([architecture.md](architecture.md)), the REST API, AQL index, and MCP server are all *derived views* over the canonical Composition store. An **EEHRxF/FHIR projection** slots in as another such consumer layer:

```
canonical Composition JSON  (source of truth)
        │
        ├── AQL index            (query)
        ├── openEHR REST API     (native interface)
        ├── MCP server           (agent access)
        └── EEHRxF/FHIR export    ← projects Compositions into FHIR resources
                                    for IPS / Patient Summary / xDHR IGs
```

This is consistent with everything else in the design: the core never changes, the projection is regenerable, and it can be added later without disturbing storage. It is a clearly-scoped, high-value future feature (roadmap Phase 6 territory), **not** an MVP commitment.

### 3. Git-portable records echo the EHDS patient-control ethos

EHDS grants patients rights to access, restrict, correct, and export their records. The repo-per-EHR topology in [versioning-and-git.md](versioning-and-git.md) - where a patient's entire record is a `git clone`-able, readable, versioned repository - is a very literal, inspectable expression of "the patient can have a copy of their whole record". This is a philosophical resonance worth noting, not a compliance claim.

---

## What anarchie does *not* claim

Intellectual honesty, consistent with the scope discipline in [scaling.md](scaling.md):

- **anarchie is not a certified EHDS EHR system.** EHDS certification is a heavyweight conformance-and-market-surveillance process for systems placed on the EU market. A teaching/experimental CDR neither needs nor should claim it.
- **anarchie is not an EEHRxF/xDHR exchange endpoint** today. The FHIR projection above is a *potential* edge feature, not a current capability, and even when built it would be a convenience projection, not a certified gateway.
- **anarchie does not implement the EHDS secondary-use regime** (Health Data Access Bodies, permits, secure processing environments). That is national-infrastructure scale.

The honest one-line framing:

> *anarchie can model, store, and version the same clinical content that the EEHRxF carries, and could project it toward EEHRxF/FHIR at its edge - but it is an experimental openEHR CDR, not a certified EHDS EHR system or exchange endpoint.*

---

## Why document this at all

Two reasons. First, anyone evaluating `anarchie` in a European context will immediately ask "how does this relate to EHDS?" - and a clear, honest answer (complementary system-of-record, not a competing exchange standard) is more credible than silence or overreach. Second, it records the **EEHRxF/FHIR projection** as a recognised future direction so that the storage and modelling decisions made now stay compatible with that edge later - the same forward-compatibility care taken for the [gitehr convergence](versioning-and-git.md).
