# anarchie commit

Commit a Composition into an EHR as an openEHR `CONTRIBUTION` - a single git
commit carrying the audit trail.

## Usage

```bash
anarchie commit <ehr> <file> [--object-id <id>] \
  [--committer <name>] [--email <email>] [-m <message>]
```

| Argument / option     | Default                | Description                                                  |
| --------------------- | ---------------------- | ------------------------------------------------------------ |
| `<ehr>`               | -                      | The EHR id to commit into.                                   |
| `<file>`              | -                      | Path to a canonical-JSON Composition file.                   |
| `--object-id <id>`    | (new object)           | Object id of an existing Composition to create a new version of. |
| `--committer <name>`  | `anarchie`             | Committer name for the audit trail.                          |
| `--email <email>`     | `anarchie@localhost`   | Committer email for the audit trail.                         |
| `-m`, `--message`     | `Commit composition`   | Contribution description (the commit subject).               |

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
