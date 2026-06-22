# anarchie index

Build or refresh the derived AQL query index from the canonical Composition
files - the read model that [anarchie aql](aql.md) and [stored queries](query.md)
run against.

## Usage

```bash
anarchie index [--rebuild]
```

| Option        | Default | Description                                          |
| ------------- | ------- | ---------------------------------------------------- |
| `--rebuild`   | off     | Drop and rebuild the entire index from scratch.      |

## Example

```bash
$ anarchie index
Indexed 1 composition(s) into /home/you/my-cdr/index/aql.db
```

The index is a single SQLite database at `<root>/index/aql.db`. The number
reported is how many head Compositions were (re-)indexed on this run.

## Freshness tracking

By default `anarchie index` only re-indexes the EHRs whose git `HEAD` has moved
since the last run. Running it again when nothing has changed does no work:

```bash
$ anarchie index
Indexed 0 composition(s) into /home/you/my-cdr/index/aql.db
```

Pass `--rebuild` to discard the existing index and rebuild every EHR from
scratch - useful after an upgrade that changes how paths are extracted, or
whenever you simply want a guaranteed-fresh read model:

```bash
$ anarchie index --rebuild
Indexed 1 composition(s) into /home/you/my-cdr/index/aql.db
```

## A derived read model (CQRS)

The index is a **read model**, separate from the system of record. The canonical
Composition files in `ehrs/` are authoritative; the index is a disposable,
query-optimised projection of them - the read side of a CQRS split.

That means the index:

- is **never authoritative** - nothing is ever read back from it as truth;
- is **`.gitignore`d** - it is regenerated locally, never committed;
- is **disposable** - you can delete `<root>/index/` at any time and rebuild it
  with `anarchie index --rebuild`, losing nothing.

Because of this, the CLI query commands ([aql](aql.md), [query run](query.md))
do not refresh the index themselves - they read whatever it currently holds, so
run `anarchie index` after new commits. Querying [over the REST API](serve.md)
is different: it refreshes the index incrementally before each query, so a
Composition committed over HTTP is immediately queryable.

## See also

- [anarchie aql](aql.md) · [anarchie query](query.md) · [anarchie serve](serve.md)
- [On-disk Format](../reference/on-disk-format.md)
