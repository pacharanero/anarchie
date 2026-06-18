# anarchie cat

Print a Composition - either its current head, or a specific historical version.

## Usage

```bash
anarchie cat <ehr> <target>
```

| Argument   | Description                                                              |
| ---------- | ----------------------------------------------------------------------- |
| `<ehr>`    | The EHR id.                                                             |
| `<target>` | An `object_id` (prints the head) or a full `version_uid` (prints that version). |

`anarchie` decides which you meant by looking for `::`: a `version_uid` contains
it (`object_id::system_id::version_tree_id`), an object_id does not.

## Print the head

```bash
anarchie cat "$EHR" 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55
```

The head version lives in the working tree, so this is a direct file read.

## Print a specific version

```bash
anarchie cat "$EHR" "9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55::anarchie.example.org::1"
```

Historical versions are reconstructed from git history with `git show
<commit>:<path>`. The working file always holds the latest version; git holds
everything before it.

## See also

- [anarchie log](log.md)
- [The Git-backed Store](../walkthrough/the-store.md)
