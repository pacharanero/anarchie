# anarchie info

Summarise an openEHR Composition stored as canonical JSON, without committing
anything.

## Usage

```bash
anarchie info <file>
```

| Argument | Description                                  |
| -------- | -------------------------------------------- |
| `<file>` | Path to a canonical-JSON Composition file.   |

## Example

```bash
$ anarchie info vitals.json
Composition: Blood pressure
  archetype:  openEHR-EHR-COMPOSITION.encounter.v1
  template:   vital_signs_encounter.v1
  rm_version: 1.1.0
  language:   en
  territory:  GB
  category:   event
  composer:   Dr Ada Lovelace
  content items: 1
  sections:      0
  entries:       1
  elements:      2
```

The section, entry, and element counts come from a real Reference Model
tree-walk: Sections recurse into their children, Observations descend into their
events, and Elements are counted at the leaves. If the file is not a
structurally valid Composition, the command fails with a descriptive error.

## See also

- [anarchie canonicalise](canonicalise.md)
- [The Reference Model](../walkthrough/reference-model.md)
