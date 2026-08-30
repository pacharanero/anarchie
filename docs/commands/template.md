# anarchie template

Manage the Operational Templates that act as the schema for a deployment.
Registering a template means Compositions that declare its `template_id` are
validated against it on every [commit](commit.md).

## Usage

```bash
anarchie template add <file>
anarchie template list
```

## anarchie template add

Register an Operational Template. `anarchie` accepts its native flattened-OPT
JSON form or a legacy ADL 1.4 `.opt` XML file exported by Archetype Designer or
the ADL Workbench. XML is lowered into the native form; the deployment stores
only native JSON.

```bash
$ anarchie template add vital_signs_encounter.opt.json
Registered template vital_signs_encounter.v1
```

The template is stored under `templates/<template_id>.opt.json` in the
deployment and added to the template index. From then on, any Composition whose
`archetype_details.template_id` matches is validated against it.

!!! note "Supported XML subset"
    XML ingestion supports COMPOSITION and archetype-root complex objects,
    single and multiple attributes, inclusive occurrence/cardinality intervals,
    `C_DV_QUANTITY` units/magnitude/precision, and `C_CODE_PHRASE` constraints.
    Quantity property constraints, defaults, rules, slots, ordered/unique
    cardinality, and other unsupported OPT features fail explicitly rather than
    weakening validation. The native JSON form stays the durable, inspectable
    template representation.

## anarchie template list

List the templates registered in the deployment.

```bash
$ anarchie template list
vital_signs_encounter.v1
```

## How templates are used

When you commit a Composition, anarchie reads its declared `template_id`. If a
template with that id is registered, its constraints are enforced alongside the
Reference Model checks. If no matching template is registered, only the
Reference Model invariants apply - so an unregistered `template_id` does not
block the commit, it simply means archetype-level constraints are not checked.

## See also

- [anarchie validate](validate.md) · [anarchie commit](commit.md)
- [Roadmap: Phase 3 - Validation](../reference/roadmap.md)
