# anarchie log

Show the version history of a Composition, derived directly from the git history
of its file.

## Usage

```bash
anarchie log <ehr> <object_id>
```

| Argument      | Description                       |
| ------------- | --------------------------------- |
| `<ehr>`       | The EHR id.                       |
| `<object_id>` | The Composition object_id.        |

## Example

```bash
$ anarchie log "$EHR" 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55
9f1c…::anarchie.example.org::2  2026-01-15T09:42:11Z  Corrected diastolic reading
  commit b2c3d4e5…
9f1c…::anarchie.example.org::1  2026-01-15T09:30:00Z  Admission observations
  commit a1b2c3d4…
```

Each entry shows the `version_uid`, the commit timestamp, the contribution
subject, and the underlying commit sha. The newest version is listed first.

## See also

- [anarchie cat](cat.md) · [anarchie diff](diff.md)
- [Inspecting the Files](../walkthrough/inspecting-the-files.md)
