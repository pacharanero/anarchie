# Querying with AQL

Storing and reading back single Compositions is useful, but the question a CDR really has to answer is a *population* one: "show me every patient whose systolic pressure was over 140." openEHR's standard query language for that is **AQL** (Archetype Query Language), and `anarchie` answers it directly over the flat files.

The trick is the same one [`sct`](https://github.com/pacharanero/sct) uses for SNOMED: **flatten once, serve many.** The canonical Composition files stay the source of truth; `anarchie` derives a disposable **index** from them and runs AQL against that. The index is a classic CQRS read model - rebuildable, never authoritative. Delete it and the patient data is untouched, because the data never lived there.

!!! note "Follow on from the store walkthrough"
    This page assumes you have a deployment with at least one committed Composition, as built in [The Git-backed Store](the-store.md).

## Build the index

```bash
anarchie index
```

```text title="output"
Indexed 1 composition(s) into index/aql.db
```

The index is a single SQLite file under `index/` (which is git-ignored - it is derived data). It flattens every Composition into a path-value table, keyed by each leaf's composition-rooted canonical path and tagged with its ENTRY archetype.

Re-running `anarchie index` is cheap: it tracks each EHR's last-indexed git HEAD and re-indexes only the records whose HEAD has moved. Pass `--rebuild` to drop and rebuild everything from scratch.

```bash
anarchie index            # incremental: only changed EHRs
anarchie index --rebuild  # full rebuild from the files
```

## Run an ad-hoc query

`anarchie aql` takes an AQL string and returns an openEHR-style ResultSet - `{q, columns, rows}`:

```bash
anarchie aql "SELECT
    o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic
  FROM COMPOSITION c
    CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]
  WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > 100"
```

```json title="output"
{
  "q": "SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic FROM COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2] WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > 100",
  "columns": [
    {
      "name": "systolic",
      "path": "o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude"
    }
  ],
  "rows": [
    [
      128.0
    ]
  ]
}
```

!!! tip "Paths are archetype-node-qualified"
    AQL identifies a leaf by its full canonical path, with the archetype node ids in brackets - `o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude`, not a friendly name like `systolic`. That is what lets the index resolve a path to an exact lookup with no JSON-walking at query time. The node ids come straight from the archetype; you can read them off a Composition with `jq`, or from the template.

Aggregates work too:

```bash
anarchie aql "SELECT COUNT(*) FROM COMPOSITION c"
```

```json title="output"
{
  "q": "SELECT COUNT(*) FROM COMPOSITION c",
  "columns": [ { "name": "#count" } ],
  "rows": [ [ 1 ] ]
}
```

The supported subset covers `SELECT` of leaf paths and aggregates (`COUNT`/`MIN`/`MAX`/`SUM`/`AVG`), `FROM … CONTAINS …`, `WHERE` (comparisons, `MATCHES`, `LIKE`, `EXISTS`, `AND`/`OR`/`NOT`), `ORDER BY`, `LIMIT`/`OFFSET`, `AS` aliases, and `$`-parameters.

## Parameterise a query

Bind `$`-parameters with `--param NAME=VALUE` (repeatable):

```bash
anarchie aql "SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic
  FROM COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]
  WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > \$threshold" \
  --param threshold=140
```

With no record above 140, the ResultSet comes back with an empty `rows` array.

## Store a query by name

Real systems do not paste AQL strings around; they register named, versioned queries. `anarchie` keeps stored queries as plain files under `queries/<name>/<version>.aql` - data, not code, so they are git-versioned alongside the templates.

```bash
echo 'SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic
FROM COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]
WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > $threshold' > high-bp.aql

anarchie query add org.example::high_systolic high-bp.aql --version 1.0.0
anarchie query list
anarchie query run org.example::high_systolic --param threshold=100
```

```text title="output"
Registered query org.example::high_systolic/1.0.0
org.example::high_systolic/1.0.0
{ "q": "…", "columns": [ … ], "rows": [ [ 128.0 ] ] }
```

`query run` defaults to the highest registered version; pass `--version` to pin one.

!!! note "The index is disposable"
    Nothing above is authoritative. `rm -rf index/ && anarchie index` rebuilds the whole read model from the canonical files. The same query path is also exposed over the [REST API](the-rest-api.md), where it refreshes the index automatically before answering - so a Composition committed over HTTP is queryable immediately, with no separate `index` step.
