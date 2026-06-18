# anarchie canonicalise

Read a Composition and re-emit it in `anarchie`'s canonical JSON form on stdout.

## Usage

```bash
anarchie canonicalise <file>
```

| Argument | Description                                  |
| -------- | -------------------------------------------- |
| `<file>` | Path to a canonical-JSON Composition file.   |

## Example

```bash
anarchie canonicalise vitals.json > vitals.canonical.json
```

## Why it matters

openEHR defines a canonical serialisation, so two systems serialising the same
Composition produce byte-comparable output. Canonicalising gives you files that
are:

- **diffable** - a meaningful change in the data is a meaningful change in the
  file;
- **hashable** - equal Compositions hash equal;
- **stable in git** - re-committing unchanged content produces an identical
  file.

`anarchie` round-trips Compositions through parse and canonicalise without
drift: run the command twice and the second output matches the first.

## See also

- [anarchie info](info.md)
- [The Reference Model](../walkthrough/reference-model.md)
