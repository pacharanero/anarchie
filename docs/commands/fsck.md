# anarchie fsck

Integrity-check the store. `fsck` reads every stored head Composition and checks
it against the Reference Model - and, where the Composition declares a
[registered template](template.md), against that template too - reporting
anything that fails to parse or conform.

## Usage

```bash
anarchie fsck [--json]
```

| Option     | Default | Description                                          |
| ---------- | ------- | ---------------------------------------------------- |
| `--json`   | off     | Emit the integrity report as JSON instead of text.   |

The command exits `0` when the store is clean and non-zero when any Composition
fails - so it drops straight into CI or a pre-flight check.

## Example

A clean store:

```bash
$ anarchie fsck
Checked 1 composition(s) across 1 EHR(s)
Store is clean.
```

When something is wrong, each problem is listed as `✗ ehr/object` with the
reason it failed to parse or conform, and the command exits non-zero.

## Why this is possible at all

Because the canonical files are the system of record - not a cache in front of a
database - the store's integrity is verifiable directly, at any time, by reading
those files. `fsck` does exactly that. It is wholly independent of the
[index](index.md): the index is a disposable read model, whereas `fsck` audits
the authoritative data itself.

## Machine-readable output

`--json` emits a report suitable for scripting, with the counts and a (here
empty) `issues` array:

```bash
$ anarchie fsck --json
{
  "ehrs": 1,
  "compositions": 1,
  "issues": []
}
```

## See also

- [anarchie validate](validate.md) · [anarchie index](index.md)
- [The Git-backed Store](../walkthrough/the-store.md)
