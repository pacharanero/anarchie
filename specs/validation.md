# Validation (RM + Template)

> **Decision: validation is reimplemented natively in Rust.** No JVM, no shelling out to Archie or the openEHR SDK at runtime. This preserves the single-binary, no-runtime promise that makes `anarchie` (like `sct`) trivial to install and run.

Validation is the price of admission for a CDR. Storing files is easy; storing only *valid* openEHR Compositions, and rejecting malformed ones at the door, is what makes `anarchie` a CDR rather than a JSON folder. It is also the single largest piece of engineering in the project.

---

## The two layers of validation

An incoming Composition must satisfy two distinct contracts:

1. **Reference Model (RM) conformance** - the Composition is structurally a valid openEHR RM instance. Every `ELEMENT` has a legal `DATA_VALUE`, `CODE_PHRASE`s are well-formed, mandatory RM attributes are present, types match. This is fixed and version-stable (RM Release 1.1.0).
2. **Operational Template (OPT) conformance** - the Composition matches the *specific* template it claims to follow: the right archetypes in the right slots, occurrences within bounds, only permitted nodes present, terminology constraints satisfied, units in range. This is per-template and data-driven.

RM validation answers "is this openEHR at all?"; OPT validation answers "is this *this kind of* openEHR?".

---

## Strategy: data-driven, not code-generated

There are two ways to validate against a template:

- **Code generation** - emit a bespoke validator per template (what some SDKs do). Fast, but means a build step per template and an explosion of generated code.
- **Interpretation** - load the Operational Template into an in-memory constraint tree and walk the Composition against it at runtime.

`anarchie` chooses **interpretation**. Templates are registered as data (`anarchie template add`), parsed once into a constraint model, and cached. Validation is a tree-walk of the Composition guided by the constraint model. This matches the file-first philosophy: templates are data, not code.

---

## Component breakdown (Rust crates/modules)

```
anarchie-rm        the Reference Model as Rust types + deserialisation
   │                (COMPOSITION, SECTION, OBSERVATION, ELEMENT, DV_* ...)
   ▼
anarchie-aom       Archetype Object Model: the constraint types
   │                (C_OBJECT, C_ATTRIBUTE, C_DV_QUANTITY, occurrences ...)
   ▼
anarchie-opt       parse an Operational Template (XML or JSON) into an AOM tree
   │
   ▼
anarchie-validate  walk a COMPOSITION against an OPT constraint tree,
                   accumulating a structured list of violations
```

- **`anarchie-rm`** is the foundation: faithful Rust structs/enums for the RM, with `serde` (de)serialisation to/from canonical JSON. This is also what the on-disk files deserialise into, so it is shared with the storage layer.
- **`anarchie-aom`** models constraints. The Archetype Object Model is the type system for "what is allowed".
- **`anarchie-opt`** turns a published OPT (Operational Templates are distributed as XML; a JSON form also exists) into an in-memory AOM tree. An OPT is already *flattened* (all specialisation and slot-filling resolved), which is a deliberate simplification: `anarchie` validates against OPTs, **not** raw ADL archetypes, so it never needs to implement archetype flattening.
- **`anarchie-validate`** is the engine: given an RM instance and an AOM tree, produce a list of violations (or an empty list = valid).

The deliberate scope cut: **`anarchie` consumes Operational Templates, it does not author or flatten archetypes.** ADL parsing, specialisation, and template flattening are the Archetype Designer's and ADL Workbench's job. By starting from a flattened OPT, the validation problem shrinks dramatically.

---

## What RM validation must check (non-exhaustive)

- Every object's `_type` is a known RM type and appears where the RM permits it.
- Mandatory attributes are present (e.g. `COMPOSITION.category`, `OBSERVATION.data`).
- `DATA_VALUE` subtypes are internally consistent (`DV_QUANTITY` has `magnitude` + `units`; `DV_CODED_TEXT` has a `defining_code`).
- `CODE_PHRASE` / terminology ids are syntactically valid.
- Cardinality and container rules from the RM (e.g. `ITEM_TREE`, `ITEM_LIST`).
- `ARCHETYPED` / `archetype_node_id` presence at archetype roots.

## What OPT validation must check (non-exhaustive)

- The archetype at each slot is one the template permits.
- Node occurrences fall within `{lower..upper}` from the constraint.
- `existence` and `cardinality` constraints on attributes.
- `C_DV_QUANTITY` property/units/magnitude-range constraints.
- `C_CODE_PHRASE` / value-set bindings - the coded value is in the allowed set.
- `C_STRING` patterns, `C_DATE_TIME` validity_kind constraints.
- Default/assumed values where the constraint supplies them.
- Mandatory nodes (`occurrences` lower bound ≥ 1) are present.

---

## Terminology validation: delegate, don't reinvent

Some constraints bind to external terminologies (SNOMED CT, LOINC, openEHR-internal value sets). Validating "is code `73211009` a valid member of this SNOMED value set?" is **out of scope for the core validator** and is delegated:

- openEHR-internal codes (the `openehr` terminology) are checked against a small built-in table.
- External terminology bindings are validated through a pluggable hook. **This is precisely where [`sct`](https://github.com/pacharanero/sct) composes**: `anarchie` can call `sct` (or any FHIR `$validate-code` endpoint) to confirm membership, then cache the result. Terminology validation is optional and configurable, so an offline `anarchie` with no terminology backend still validates structure.

---

## Validation outcomes

`anarchie validate` and the commit path produce a structured result, not just a boolean:

```jsonc
{
  "valid": false,
  "violations": [
    {
      "severity": "error",
      "rm_path": "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data/events[at0006]/data/items[at0004]/value/magnitude",
      "constraint": "C_DV_QUANTITY",
      "message": "magnitude 400 exceeds permitted range [0..300] mmHg"
    }
  ]
}
```

- **Errors** block a commit.
- **Warnings** (e.g. an assumed value was applied, an optional recommended node is absent) do not.
- Paths use the canonical openEHR path syntax so violations are addressable and machine-consumable - important for the MCP/LLM layer, which can feed a violation straight back to an agent for correction.

---

## Testing strategy

Validation correctness is non-negotiable, so it is anchored to external truth rather than our own assumptions:

- **The openEHR conformance test suite / CDR test data** - reuse the community's published valid and invalid Composition examples.
- **Cross-check against Archie** at *development* time (not runtime): a test harness runs the same Composition through both Archie and `anarchie-validate` and asserts the verdicts agree. Disagreements are bugs to investigate. This gives us a JVM oracle without a JVM dependency in the shipped binary.
- **Property-based tests** that generate RM instances from a template and assert they validate, and mutate them to assert they fail.

---

## Why this is worth the effort

Reimplementing RM/OPT validation in Rust is a multi-month undertaking and duplicates what Archie already does well. The justification is the same as `sct`'s justification for not depending on a terminology server: **the value of the project *is* the absence of the heavy runtime.** An `anarchie` that needs a JVM to validate is just EHRbase with extra steps. An `anarchie` that validates in a 10MB static binary is a genuinely new and useful thing - installable with a one-line `curl | sh`, runnable in CI, embeddable in an MCP tool, and teachable.
