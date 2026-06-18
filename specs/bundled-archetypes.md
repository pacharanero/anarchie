# Bundled Archetypes — "Batteries Included" openEHR

> The thesis: a major reason openEHR adoption stalls is that a newcomer downloads a CDR and faces an *empty* repository with no models, no templates, and a steep authoring learning curve before they can store a single blood pressure. `anarchie` should ship with a **curated, working set of Operational Templates** so that `anarchie init` gives you a CDR that can store real clinical data *immediately*.

This mirrors `sct`'s philosophy of producing a usable artefact out of the box, and SQLite's "it just works with no setup" ergonomics. It is arguably the single most adoption-friendly thing the project could do.

---

## Is it allowed? (Licensing)

**Yes - the openEHR International (CKM) archetypes are explicitly licensed for redistribution and derivative use.** But the detail matters, so it is stated precisely here.

There are *two different licences* in the openEHR world, and they are often conflated (the full four-layer picture - code, specs, archetypes, terminology - is in [licensing.md](licensing.md)):

| Material | Licence | Can we bundle/derive? |
|---|---|---|
| openEHR **specifications** (RM, AOM, ADL, OPT docs) | CC-BY-**ND** 3.0 (NoDerivs) | We don't need to - we *implement* them, we don't redistribute them. NoDerivs is irrelevant to an implementation. |
| openEHR **CKM clinical archetypes & templates** (international) | CC-BY-**SA** 3.0 (ShareAlike) | **Yes.** Redistribution and derivatives are permitted with attribution and share-alike. |

The crucial one is **CC-BY-SA 3.0** on the clinical models. ShareAlike permits exactly what bundling needs:

- **Redistribute** the archetypes/OPTs alongside `anarchie`. ✅
- **Create derivatives** - and generating an Operational Template from archetypes + a template *is* a derivative/transformation. ✅
- **Conditions**: attribute the openEHR Foundation (and original authors), and license the bundled models themselves under CC-BY-SA. ✅ (not the *code* - see below).

### Practical licensing requirements

1. **Dual-license the repo.** The `anarchie` *code* is under **AGPL-3.0-or-later**. The *bundled archetypes/OPTs* live in their own directory under **CC-BY-SA 3.0**, with a clear `LICENSE` and `NOTICE` in that directory. This separation is standard practice (it is how `sct` keeps SNOMED *data* licensing distinct from its *code* licensing, and how many projects ship CC data with permissive code). The full approach is specified in [licensing.md](licensing.md).
2. **Attribution / provenance.** Ship a `NOTICE`/`ATTRIBUTION.md` listing each archetype, its CKM identifier, version, author/custodian, and the CC-BY-SA notice. CKM archetypes already carry rich `description`/`original_author`/`copyright` metadata in their ODIN header - preserve it; do not strip it.
3. **Stick to openEHR International (CKM) published archetypes.** National programmes (e.g. Norway, UK NHS, Catalonia) publish under their own namespaces and *may* attach different terms. The internationally-governed CKM content under the `openEHR` namespace is the safe, clearly CC-BY-SA set. Prefer **Published** (not Draft) archetypes for the default bundle.
4. **Terminology is NOT bundled.** An archetype contains terminology *bindings* (references to SNOMED CT / LOINC codes), not the terminology *content*. Shipping archetypes therefore does **not** redistribute SNOMED CT or LOINC, and does not require their licences. *Validating* those bindings at runtime is a separate, optional step delegated to a terminology backend (e.g. [`sct`](https://github.com/pacharanero/sct)), for which the operator supplies their own SNOMED licence. This keeps `anarchie` itself free of terminology-licensing entanglements.

> ⚠️ **Verify before shipping.** The CKM "Terms of Use" / "Editorial and Governance" pages should be re-read at packaging time and the exact CC-BY-SA version and wording quoted in the NOTICE. Licensing wording can change; this document records the position as understood at design time and must not be treated as legal advice.

---

## Is it possible? (Technical)

Yes, and it fits the architecture cleanly because **`anarchie` consumes Operational Templates** ([validation.md](validation.md)). The bundle is just a set of pre-built `.opt.json` files dropped into the `templates/` directory ([on-disk-format.md](on-disk-format.md)).

### Build pipeline for the bundle (done once, at `anarchie` build time - not by the end user)

```
CKM published archetypes (.adl / ADL2)
        │
        │  author a curated template per use-case
        ▼
templates (.oet / ADL2 template)            ← hand-curated, version-controlled in-repo
        │
        │  flatten with an existing tool
        │  (Archetype Designer / ADL Workbench / Archie)
        ▼
Operational Templates (.opt → canonical .opt.json)   ← the shipped artefact
        │
        ▼
bundled into anarchie's  templates/  directory
```

The flattening uses *existing* tooling (Archetype Designer, ADL Workbench, or Archie) as a **build-time** step - it does not add a runtime dependency, exactly as Archie is used only as a *test-time* oracle for validation. The end user never authors or flattens anything; they receive ready-to-use OPTs.

### Distribution mechanics

- The bundle ships inside the binary's release artefacts (or as a small companion archive), under `templates/`.
- `anarchie init --with-starter-templates` (default on) populates a new deployment with the bundled OPTs.
- A `--minimal` flag gives an empty CDR for those who want to bring their own templates.
- Bundled OPTs are versioned and carry a manifest mapping each OPT to its source archetype versions and CKM provenance.

---

## Which archetypes? The proposed starter set

The organising principle: **make the default bundle sufficient to represent an [International Patient Summary (IPS)](https://international-patient-summary.net/) -style record**, plus the universal vital-signs encounter that every demo needs. IPS is a deliberate choice - it is an internationally agreed *minimal but clinically meaningful* dataset, which is exactly the right target for "batteries included". It answers "what should a general-purpose CDR be able to store on day one?" with an existing, governed answer.

### Tier 1 — Core (the default `init` set)

Built from **Published** openEHR International archetypes:

| Use case | Template (OPT) | Key archetypes (openEHR International) |
|---|---|---|
| Vital signs encounter | `vital-signs-encounter` | `OBSERVATION.blood_pressure`, `OBSERVATION.pulse`, `OBSERVATION.body_temperature`, `OBSERVATION.respiration`, `OBSERVATION.pulse_oximetry`, `OBSERVATION.body_weight`, `OBSERVATION.height`, `OBSERVATION.body_mass_index` |
| Problem list | `problem-list` | `EVALUATION.problem_diagnosis`, `CLUSTER.problem_qualifier` |
| Medication list | `medication-list` | `INSTRUCTION.medication_order`, `EVALUATION.medication_statement` (Medication management family) |
| Allergies & adverse reactions | `adverse-reaction-list` | `EVALUATION.adverse_reaction_risk` |
| Laboratory result report | `laboratory-result-report` | `OBSERVATION.laboratory_test_result`, `CLUSTER.laboratory_test_analyte`, `CLUSTER.specimen` |
| Procedures | `procedure-list` | `ACTION.procedure` |
| Clinical encounter note | `encounter-note` | `COMPOSITION.encounter`, `EVALUATION.clinical_synopsis`, `SECTION.*` |
| Immunisations | `immunisation-list` | `ACTION.medication` (vaccine) / `OBSERVATION.*` as per IPS |

These eight templates roughly span the IPS content sections (problems, medications, allergies, results, procedures, immunisations, vitals) plus the encounter scaffold - enough that a newcomer can store a believable patient record immediately.

### Tier 2 — Extended (opt-in, `--with-extended`)

- `OBSERVATION.symptom_sign`, `OBSERVATION.story` / `EVALUATION.history` (clinical narrative)
- `CLUSTER.anatomical_location`, `CLUSTER.timing_*` (common reusable clusters)
- `EVALUATION.exclusion_*` (no known allergies / no known problems - clinically important negatives)
- `OBSERVATION.lab_test-*` specialisations (HbA1c, lipids, FBC) for realistic lab demos
- `COMPOSITION.report`, `COMPOSITION.health_summary`

### Explicitly out of the default bundle

- **Draft / in-review archetypes** - unstable; opt-in only.
- **National-namespace archetypes** - licensing/terms vary; let users add their own.
- **Demographic model archetypes** (PARTY/person) - subject references are stored opaquely in the MVP ([architecture.md](architecture.md)).
- **Specialist domain sets** (oncology staging, ophthalmology biometry, etc.) - available as separate downloadable packs, not the default.

---

## A possible "archetype pack" ecosystem (future)

Beyond the built-in starter set, the bundling mechanism naturally extends to **installable packs**, echoing `sct`'s codelist/refset model and the sibling [Knowledge Artefacts Package Manager](https://github.com/pacharanero/knowledge-artefacts-package-manager) (`kam`) in this very workspace:

```
anarchie pack add ips           # the International Patient Summary set (default)
anarchie pack add lab-extended  # richer laboratory analytes
anarchie pack add nhs-uk        # a national pack (user accepts that pack's terms)
anarchie pack list
```

Each pack is a versioned set of OPTs plus a provenance/licence manifest. This is where `anarchie` could genuinely move the needle on the "empty CDR" adoption problem: not just batteries included, but a clean way to add more batteries. It also dovetails with `kam`, which already exists to manage openEHR knowledge artefacts - `anarchie` could consume `kam` packages directly rather than reinventing packaging.

---

## Why this matters

The "lack of a batteries-included openEHR" is a real adoption barrier: the standard is powerful but the on-ramp is "now go author some archetypes". Shipping a curated, correctly-licensed, IPS-aligned OPT set turns `anarchie init` into a CDR that stores real clinical data in the first five minutes - the openEHR equivalent of `sqlite3 test.db` just working. Combined with the single-binary, no-runtime promise, that is a meaningfully lower barrier to trying openEHR than anything currently available.
