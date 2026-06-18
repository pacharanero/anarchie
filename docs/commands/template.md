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

Register an Operational Template. The file is anarchie's native flattened-OPT
JSON: a `template_id`, a root `concept` archetype id, and an Archetype Object
Model `definition` tree.

```bash
$ anarchie template add vital_signs_encounter.opt.json
Registered template vital_signs_encounter.v1
```

The template is stored under `templates/<template_id>.opt.json` in the
deployment and added to the template index. From then on, any Composition whose
`archetype_details.template_id` matches is validated against it.

!!! note "Native JSON form"
    Templates are anarchie's own flattened-JSON representation of the AOM.
    Ingesting `.opt` XML exported from Archetype Designer or the ADL Workbench is
    planned future work; for now you author or generate the JSON form directly.

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
