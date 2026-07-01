# Licensing

`anarchie` combines four kinds of material, each governed by a different licence. Conflating them is the most common licensing mistake in the openEHR ecosystem, so this document fixes the approach explicitly and tells contributors exactly how to label each part. **This is design-time understanding, not legal advice** - verify wording against the upstream sources at packaging time.

---

## The four layers and their licences

| Layer | What it is | Licence | Why |
|---|---|---|---|
| **Code** | The `anarchie` Rust binary and all source we write | **AGPL-3.0-or-later** | Strong copyleft: a hosted/networked CDR must share its source, keeping the project and its derivatives open. |
| **openEHR specifications** | RM, AOM, ADL, OPT, AQL normative docs | **CC-BY-ND 3.0** (NoDerivs) — *not redistributed* | We *implement* the specs; we never ship modified copies of them. NoDerivs therefore does not constrain us. |
| **Clinical archetypes / OPTs** | The bundled openEHR International models ([bundled-archetypes.md](bundled-archetypes.md)) | **CC-BY-SA 3.0** (ShareAlike) | ShareAlike explicitly permits redistribution and derivatives (OPT generation), with attribution and share-alike. |
| **Terminology** | SNOMED CT, LOINC, ICD code *content* | **Not bundled at all** | We ship only the *bindings* (code references inside archetypes), never the terminology content. The operator brings their own terminology licence. |

---

## 1. Code — AGPL-3.0-or-later

- All first-party source code is licensed **AGPL-3.0-or-later**.
- A `LICENSE` file at the repository root contains the AGPL-3.0 text; source files carry the standard SPDX header:

  ```rust
  // SPDX-License-Identifier: AGPL-3.0-or-later
  ```
- **Rationale:** a CDR is exactly the kind of software that is often run as a network service. AGPL's network-use clause (§13) ensures that anyone offering `anarchie` over a network must make their (possibly modified) source available to users. For a project whose whole point is openness and inspectability, this is the right default.
- **Dependency compatibility:** AGPL-3.0 is compatible with permissively-licensed Rust crates (MIT/Apache-2.0/BSD), which is the bulk of the crates.io ecosystem. **Avoid** pulling in dependencies under incompatible copyleft (e.g. some GPL-2.0-only crates) - check at `cargo add` time. Record the dependency-licence audit (e.g. via `cargo-deny`) in CI.
- **Test-time oracles** (Archie, used only to cross-check validation - see [validation.md](validation.md)) are **not** linked into or shipped with the binary, so their JVM/Apache-2.0 licensing does not affect distribution. They are dev-dependencies only.

## 2. openEHR specifications — CC-BY-ND 3.0 (implemented, never redistributed)

- The RM/AOM/ADL/OPT/AQL specifications are licensed **CC-BY-ND 3.0 (NoDerivs)**.
- NoDerivs forbids distributing *modified versions of the specification documents*. It does **not** forbid implementing them - copyright protects the document's expression, not the ideas or the interoperability interface it describes.
- **Therefore:** `anarchie` *implements* these specifications in original code. We do **not** copy spec prose, diagrams, or normative tables into the repo. Where we need to reference a spec, we **link** to the canonical `specifications.openehr.org` URL and cite it; we do not vendor the text.
- Any short quotation for explanatory purposes stays within fair-dealing/fair-use limits, is clearly attributed, and is never a wholesale reproduction.

## 3. Archetypes / OPTs — CC-BY-SA 3.0 (bundled as data, segregated from code)

- Bundled clinical models live in a **dedicated directory** (e.g. `templates/` / `archetypes/`) with their **own** `LICENSE` (CC-BY-SA 3.0) and a `NOTICE`/`ATTRIBUTION.md`. They are **not** under the code's AGPL licence.
- The directory's `NOTICE` lists, per artefact: CKM identifier, version, original author/custodian, copyright statement (preserved verbatim from the archetype's ODIN header), and the CC-BY-SA 3.0 notice.
- **ShareAlike obligation:** the bundled models (and any modifications/derived OPTs we generate) are themselves licensed CC-BY-SA 3.0. ShareAlike applies to *the models*, not to the `anarchie` *code* that processes them - the two are independent works distributed together (mere aggregation), which is why the directory separation matters.
- **Scope:** default bundle is **openEHR International, Published** archetypes only. National-namespace archetypes (which may carry different terms) are excluded from the default and added by users/packs at their own discretion.
<!-- REUSE-IgnoreStart -->
- **SPDX:** data files carry `SPDX-License-Identifier: CC-BY-SA-3.0` in an accompanying manifest where inline headers are not appropriate.
<!-- REUSE-IgnoreEnd -->


## 4. Terminology — explicitly NOT bundled

This must be signposted loudly because it is the most common source of licensing fear around openEHR distributions.

- An archetype contains terminology **bindings** - references such as "this element's value is constrained to SNOMED CT code `271649006`". It does **not** contain the SNOMED CT, LOINC, or ICD **content** (the concept tables, descriptions, hierarchies).
- **Shipping archetypes therefore does not redistribute any terminology**, and triggers none of SNOMED International's / Regenstrief's / WHO's licences.
- **Runtime binding validation is optional and the operator's responsibility.** If an operator wants `anarchie` to confirm that a coded value is a valid member of a bound value set, they configure a terminology backend (e.g. [`sct`](https://github.com/pacharanero/sct) for SNOMED CT, or a FHIR `$validate-code` endpoint) **for which they hold the appropriate licence**. With no backend configured, `anarchie` validates structure only and treats bindings as opaque.
- The README and the bundle NOTICE both state plainly: *"anarchie bundles archetype terminology bindings only. It does not include, and you must separately license, any clinical terminology (SNOMED CT, LOINC, ICD, etc.)."*

---

## Repository layout for licensing clarity

```
anarchie/
├── LICENSE                      ← AGPL-3.0-or-later (the code)
├── NOTICE                       ← top-level: explains the multi-licence structure + terminology disclaimer
├── README.md                    ← "Licensing" section summarising all four layers
├── src/ …                       ← AGPL-3.0-or-later, SPDX headers
└── templates/                   ← bundled models
    ├── LICENSE                  ← CC-BY-SA-3.0 (the archetypes/OPTs)
    ├── ATTRIBUTION.md           ← per-artefact provenance + copyright
    └── *.opt.json               ← the models themselves
```

This mirrors how `sct` keeps its *code* licensing distinct from the *data* (SNOMED) it processes, and follows the widely-used REUSE / SPDX convention for multi-licence repositories (see the `REUSE.toml` pattern `sct` itself uses).

---

## Required statements at distribution time

Every release must:

1. Ship the root **AGPL-3.0-or-later** `LICENSE` and the `templates/` **CC-BY-SA-3.0** `LICENSE`.
2. Ship a top-level **`NOTICE`** that:
   - explains the code/specs/archetypes/terminology split in plain language,
   - attributes the openEHR Foundation for the bundled archetypes,
   - carries the **terminology-not-bundled** disclaimer verbatim.
3. Ship `templates/ATTRIBUTION.md` with per-archetype provenance.
4. Pass the CI dependency-licence audit (`cargo-deny` or equivalent) confirming no AGPL-incompatible dependency crept in.
5. Re-confirm the upstream CC-BY-SA version/wording on the CKM Terms of Use page and quote it in the bundle `NOTICE`.

---

## Summary statement (for the README)

> **anarchie licensing in one paragraph.** The `anarchie` software is licensed **AGPL-3.0-or-later**. It *implements* the openEHR specifications (which are CC-BY-ND and are not redistributed here). It *bundles* openEHR International clinical archetypes/templates under their original **CC-BY-SA 3.0** licence, kept in a separate directory with full attribution. It bundles **only terminology bindings, never terminology content** - SNOMED CT, LOINC, ICD and similar must be licensed and supplied separately by the operator if runtime terminology validation is wanted.
