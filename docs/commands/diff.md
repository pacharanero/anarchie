# anarchie diff

Diff two versions of a Composition. Because the stored files are canonical, the
diff shows exactly what changed clinically - plus the `version_uid` bump - and
nothing else.

## Usage

```bash
anarchie diff <ehr> <object_id> <from> <to>
```

| Argument      | Description                          |
| ------------- | ------------------------------------ |
| `<ehr>`       | The EHR id.                          |
| `<object_id>` | The Composition object_id.           |
| `<from>`      | The earlier `version_tree_id` (1-based). |
| `<to>`        | The later `version_tree_id` (1-based).   |

`version_tree_id`s are 1-based; version `0` does not exist and is rejected.

## Example

```bash
anarchie diff "$EHR" 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55 1 2
```

This diffs version 1 against version 2 of the Composition.

## See also

- [anarchie log](log.md)
- [The Reference Model](../walkthrough/reference-model.md)
