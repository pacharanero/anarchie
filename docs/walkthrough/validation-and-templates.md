# Validation and Templates

Up to this point `anarchie` will store any well-formed Composition you hand it.
That makes it a versioned JSON folder, not yet a clinical data repository. The
difference is **validation**: a real CDR rejects data that does not conform to
its schema, at the door, before it is ever written. This walkthrough adds that.

## Two layers of conformance

openEHR validation has two distinct layers, and `anarchie` treats them
separately because they answer different questions.

The **Reference Model** layer asks: *is this a structurally valid openEHR
Composition at all?* These are the invariants that hold for every Composition
regardless of which archetype it uses - an `ELEMENT` must carry either a value
or a null-flavour, a `CODE_PHRASE` must name a terminology and a code, a
`DV_QUANTITY` must have units. `anarchie` checks these against the **typed**
Reference Model structs, so the rules are expressed in Rust's type system as far
as possible and the remaining invariants are a direct tree-walk.

The **Operational Template** layer asks: *does this Composition conform to the
specific clinical content model it claims to follow?* Is the systolic blood
pressure in `mm[Hg]` and within a plausible range? Is the mandatory observation
actually present? These constraints come from an archetype/template, expressed
in the **Archetype Object Model** (AOM).

## The template is already registered

A template is the schema. `anarchie init` already installed the IPS-aligned
starter set, so the one this walkthrough needs -
`vital_signs_encounter.v1` - is registered from the start:

```bash
$ anarchie template list
adverse_reaction_list.v1
encounter_note.v1
immunisation_list.v1
laboratory_result_report.v1
medication_list.v1
problem_list.v1
procedure_list.v1
vital_signs_encounter.v1
```

Each template is stored inside the deployment under `templates/` and indexed.
Any Composition whose `template_id` is one of these is validated against it
automatically. To add your *own* Operational Template, register its
anarchie-OPT JSON with `anarchie template add <your-template>.opt.json`; it
joins the list the same way.

!!! note "anarchie's native template form"
    The file above is anarchie's own flattened-OPT JSON: a `template_id`, a root
    archetype `concept`, and an AOM `definition` tree of `C_COMPLEX_OBJECT`
    nodes, attributes, and leaf constraints. Importing `.opt` XML from Archetype
    Designer is planned; for now the JSON form is authored or generated directly.

## Validating without committing

Use `anarchie validate` to check a file in isolation. With no template it runs
the Reference Model checks; with `--template` it adds the archetype constraints:

```bash
$ anarchie validate vitals.json --template vital_signs_encounter.v1
valid
```

When something is wrong, each violation reports a severity, the openEHR path to
the offending node, the constraint that failed, and a human-readable message.
Make a deliberately broken copy first - push the systolic magnitude (128) far
out of physiological range:

```bash
sed 's/128.0/5000.0/' vitals.json > bad.json
```

```bash
$ anarchie validate bad.json --template vital_signs_encounter.v1
  [error] /content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude (C_DV_QUANTITY)
        magnitude 5000 outside permitted range for units "mm[Hg]"
```

Add `--format json` for machine-readable output you can pipe into CI.

## Validation at the door

The point of all this is that you no longer have to remember to validate.
**Every commit is validated automatically.** A Composition with an
error-level violation is rejected and nothing is written to the repository:

```bash
$ anarchie commit "$EHR" bad.json -m "oops"
Rejected: composition failed validation
  [error] /content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude (C_DV_QUANTITY)
        magnitude 5000 outside permitted range for units "mm[Hg]"
Error: 1 validation error(s)
```

The git history stays clean because the bad data never reached it. If you need
to bypass the check - importing legacy data you intend to clean up, say - pass
`--no-validate`.

## Why split typed-RM from JSON-guided-OPT?

A design note worth surfacing, because it shaped the code. Reference Model
checks walk the typed structs; template checks walk the **canonical JSON**
guided by the AOM tree. That asymmetry is deliberate: the AOM names Reference
Model attributes as plain strings (`"content"`, `"data"`, `"events"`) that map
one-to-one onto JSON keys, so following an archetype constraint tree over a
`serde_json::Value` is dramatically simpler than reflecting over typed enums.
The typed tree is the right tool for universal invariants; the JSON tree is the
right tool for archetype-specific constraints. Using each where it fits keeps
both validators small.

## Where this leaves us

`anarchie` is now a real CDR: durable, versioned, inspectable - and conformant.
What it cannot yet do is answer questions *across* records ("show me every
systolic over 140"). That is querying, and it is the subject of the next phase
of the [roadmap](../reference/roadmap.md).
