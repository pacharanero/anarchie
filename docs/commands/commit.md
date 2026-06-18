# anarchie commit

Commit a Composition into an EHR as an openEHR `CONTRIBUTION` - a single git
commit carrying the audit trail.

## Usage

```bash
anarchie commit <ehr> <file> [--object-id <id>] [--no-validate] \
  [--committer <name>] [--email <email>] [-m <message>]
```

| Argument / option     | Default                | Description                                                  |
| --------------------- | ---------------------- | ------------------------------------------------------------ |
| `<ehr>`               | -                      | The EHR id to commit into.                                   |
| `<file>`              | -                      | Path to a canonical-JSON Composition file.                   |
| `--object-id <id>`    | (new object)           | Object id of an existing Composition to create a new version of. |
| `--no-validate`       | off                    | Skip validation and commit the Composition unchecked.        |
| `--committer <name>`  | `anarchie`             | Committer name for the audit trail.                          |
| `--email <email>`     | `anarchie@localhost`   | Committer email for the audit trail.                         |
| `-m`, `--message`     | `Commit composition`   | Contribution description (the commit subject).               |

## Validation

Every commit is validated before it is written. Reference Model invariants are
always checked; if the Composition declares a `template_id` that has been
[registered](template.md), its Operational Template constraints are enforced
too. A Composition with any **error**-level violation is rejected and nothing is
written:

```bash
$ anarchie commit "$EHR" bad.json -m "oops"
Rejected: composition failed validation
  [error] /content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/.../value/magnitude (C_DV_QUANTITY)
        magnitude 5000 outside permitted range for units "mm[Hg]"
Error: 1 validation error(s)
```

Use `--no-validate` to bypass the check (for example when importing legacy data
you intend to clean up later). See [anarchie validate](validate.md) to check a
file without committing.

## New Composition

Omit `--object-id` to store a brand-new Composition. `anarchie` assigns a fresh
object_id, and the version is `1`. The change is recorded as a `Creation`.

```bash
$ anarchie commit "$EHR" vitals.json -m "Admission observations"
Committed 9f1c…::anarchie.example.org::1
  object_id:       9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55
  contribution_id: 4d8e…
  commit:          a1b2c3d4…
```

## New version of an existing Composition

Pass `--object-id` to add a version to an existing Composition. The
`version_tree_id` increments, and the change is recorded as a `Modification`.

```bash
$ anarchie commit "$EHR" vitals-updated.json \
    --object-id 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55 \
    -m "Corrected diastolic reading"
Committed 9f1c…::anarchie.example.org::2
```

## What gets written

- The canonical Composition file (the new head, in the working tree).
- A **contribution manifest** describing the version set and audit.
- One git commit whose author/committer/timestamp are the openEHR
  `AUDIT_DETAILS`, with trailers `anarchie-contribution-id`,
  `anarchie-change-type`, and `anarchie-system-id`.

The `version_uid` has the form `object_id::system_id::version_tree_id`.

## See also

- [anarchie cat](cat.md) · [anarchie log](log.md) · [anarchie diff](diff.md)
- [Why git?](../why/why-git.md)
