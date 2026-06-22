# The Git-backed Store

This is where `anarchie` earns its name. Each patient record is its own git
repository, and storing a clinical document is an audited git commit. You never
have to touch git yourself - `anarchie` drives it - but everything it does is a
plain git operation you can inspect afterwards.

## Create an EHR

```bash
$ anarchie ehr new
1b4e28ba-2fa1-11d2-883f-0016d3cca427
```

The command prints the new EHR id (a UUID). Behind it, `anarchie` created
`ehrs/<id>/`, ran `git init` inside it, wrote the `EHR` and `EHR_STATUS`
objects, and made the first commit - the creation audit.

List the EHRs in the deployment at any time:

```bash
$ anarchie ehr list
1b4e28ba-2fa1-11d2-883f-0016d3cca427
```

You can attach an audit identity to the creation:

```bash
anarchie ehr new --committer "Dr Ada Lovelace" --email ada@example.org
```

## Commit a Composition

Storing a Composition is a `CONTRIBUTION`, which `anarchie` maps onto a single
git commit:

```bash
$ EHR=1b4e28ba-2fa1-11d2-883f-0016d3cca427
$ anarchie commit "$EHR" vitals.json \
    --committer "Dr Ada Lovelace" --email ada@example.org \
    -m "Admission observations"
Committed 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55::anarchie.example.org::1
  object_id:       9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55
  commit:          a1b2c3d4…
  contribution_id: 4d8e…
```

What just happened:

- `anarchie` assigned a fresh **object_id** to this Composition.
- It built the **version_uid** as `object_id::system_id::version_tree_id`. This
  is the first version, so the `version_tree_id` is `1`.
- It wrote the canonical Composition file plus a **contribution manifest**, then
  made one git commit whose author, committer, and timestamp are the openEHR
  `AUDIT_DETAILS`, and whose trailers (`anarchie-contribution-id`,
  `anarchie-change-type`, `anarchie-system-id`) tie the commit back to the
  openEHR model.

!!! info "Why the manifest does not contain its own commit"
    A `CONTRIBUTION` references the commit that recorded it - but a commit cannot
    contain its own hash. So the link runs the other way: the commit carries an
    `anarchie-contribution-id` trailer, and the manifest stays clean.

## Commit a new version

Pass `--object-id` to create a new version of an *existing* Composition rather
than a brand-new one:

```bash
$ anarchie commit "$EHR" vitals-updated.json \
    --object-id 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55 \
    --committer "Dr Ada Lovelace" --email ada@example.org \
    -m "Corrected diastolic reading"
Committed 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55::anarchie.example.org::2
```

The `version_tree_id` is now `2`. The change is recorded as a `Modification`
rather than a `Creation` in the audit.

## Read it back

`anarchie cat` prints the **head** version when given an object_id, or a
**specific** version when given a full version_uid:

```bash
# the current head
anarchie cat "$EHR" 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55

# version 1, reconstructed from git history
anarchie cat "$EHR" "9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55::anarchie.example.org::1"
```

The head lives in the working tree; older versions are reconstructed with
`git show <commit>:<path>`. The working file always holds the latest version,
and git holds everything before it.

## See the history

```bash
$ anarchie log "$EHR" 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55
9f1c…::anarchie.example.org::2  2026-01-15T09:42:11Z  Corrected diastolic reading
  commit b2c3d4e5…
9f1c…::anarchie.example.org::1  2026-01-15T09:30:00Z  Admission observations
  commit a1b2c3d4…
```

That is the version history of the Composition, derived directly from
`git log` of its file.

## Diff two versions

```bash
$ anarchie diff "$EHR" 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55 1 2
```

`from` and `to` are 1-based `version_tree_id`s. Because the files are canonical,
the diff shows exactly what changed clinically - plus the `version_uid` bump -
and nothing else.

[:octicons-arrow-right-24: Next: Inspecting the Files](inspecting-the-files.md)
