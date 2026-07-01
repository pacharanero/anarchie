# anarchie validate

Check a Composition against the openEHR Reference Model - and, optionally,
against a registered Operational Template - without committing anything.

## Usage

```bash
anarchie validate <file> [--template <template_id>] [--format json]
```

| Argument / option        | Default | Description                                                       |
| ------------------------ | ------- | ----------------------------------------------------------------- |
| `<file>`                 | -       | Path to a canonical-JSON Composition file.                        |
| `--template <id>`        | (none)  | Also validate against this [registered template](template.md).    |
| `--format <fmt>`         | `text`  | Global flag: `text` (default) or `json` for the structured report. |

The command exits `0` when there are no errors and `1` when at least one
error-level violation is found. Warnings do not affect the exit code.

## What gets checked

**Reference Model** invariants are always checked - the rules that hold for
*every* openEHR Composition regardless of archetype:

- `rm_version` is present.
- Every `CODE_PHRASE` carries a terminology and a code.
- Every `ELEMENT` has exactly one of `value` or `null_flavour`.
- Every `DV_QUANTITY` has units and a valid `magnitude_status` (`=`, `<`, `>`,
  `<=`, `>=`, `~`).
- Every `DV_PROPORTION` has a valid `type` and a non-zero denominator.

**Operational Template** constraints are checked when you pass `--template`:
occurrences, existence and cardinality, plus leaf constraints for
`C_DV_QUANTITY` (permitted units and magnitude range), `C_CODE_PHRASE`
(terminology and code set), `C_STRING` (permitted values) and `C_DV_ORDINAL`.

## Examples

A clean Composition:

```bash
$ anarchie validate vitals.json
valid
```

Validated against its template, with a breach:

```bash
$ anarchie validate bad.json --template vital_signs_encounter.v1
  [error] /content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/.../value/magnitude (C_DV_QUANTITY)
        magnitude 5000 outside permitted range for units "mm[Hg]"
```

Machine-readable output for scripting or CI:

```bash
$ anarchie validate bad.json --template vital_signs_encounter.v1 --format json
{
  "valid": false,
  "violations": [
    {
      "severity": "error",
      "rm_path": "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/.../value/magnitude",
      "constraint": "C_DV_QUANTITY",
      "message": "magnitude 5000 outside permitted range for units \"mm[Hg]\""
    }
  ]
}
```

## See also

- [anarchie template](template.md) · [anarchie commit](commit.md)
- [The Reference Model](../walkthrough/reference-model.md)
