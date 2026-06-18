# The Reference Model

Before storing anything, `anarchie` has to understand it. The
`anarchie-rm` crate is a native Rust model of the openEHR Reference Model with a
**canonical JSON** reader and writer. Two features fall straight out of it and
are useful on their own, without a repository.

## Inspecting a Composition

Point `anarchie info` at any canonical-JSON Composition and it parses the whole
tree and summarises it:

```bash
$ anarchie info vitals.json
Composition: Blood pressure
  archetype:  openEHR-EHR-COMPOSITION.encounter.v1
  template:   blood_pressure
  rm_version: 1.1.0
  language:   en
  territory:  GB
  category:   event
  composer:   Dr A. Clinician
  content items: 1
  sections:      0
  entries:       1
  elements:      4
```

Those counts come from an actual tree-walk of the Reference Model: Sections
recurse, Observations descend into their events, and Elements are counted at the
leaves. If the file is not a structurally valid Composition, `info` fails with a
clear error rather than guessing.

## Canonicalising

`anarchie canonicalise` reads a Composition and re-emits it in `anarchie`'s
canonical JSON form:

```bash
anarchie canonicalise vitals.json > vitals.canonical.json
```

This matters because openEHR defines a **canonical serialisation**: two systems
that serialise the same Composition should produce byte-comparable output. That
property is the foundation everything else rests on:

- **Diffable.** Canonical files diff cleanly, so a meaningful change in the data
  produces a meaningful change in the file.
- **Hashable.** Equal Compositions hash equal, so de-duplication and integrity
  checks are trivial.
- **Stable in git.** Re-committing unchanged content produces an identical file,
  so the only thing that changes between two versions is what *actually*
  changed.

!!! note "Round-trip stability"
    `anarchie` is tested to round-trip Compositions through parse and
    canonicalise without drift. Run `canonicalise` twice and the second output
    is identical to the first.

[:octicons-arrow-right-24: Next: The Git-backed Store](the-store.md)
