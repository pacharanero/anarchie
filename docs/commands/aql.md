# anarchie aql

Run an ad-hoc AQL query against the [index](index.md) and print an openEHR-style
ResultSet as JSON.

## Usage

```bash
anarchie aql "<query>" [--param NAME=VALUE]...
```

| Argument / option        | Default | Description                                                  |
| ------------------------ | ------- | ------------------------------------------------------------ |
| `<query>`                | -       | The AQL query text (quote it - it contains spaces).          |
| `--param NAME=VALUE`     | (none)  | Bind a `$`-parameter. Repeatable.                            |

The result is an openEHR ResultSet object `{q, columns, rows}`: `q` echoes the
query, `columns` describes each projected column (its `name`, and `path` for a
leaf projection), and `rows` is the tabular data.

!!! note "Index first"
    `anarchie aql` reads the index as it currently stands; it does not refresh
    it. Run [anarchie index](index.md) after new commits, or query
    [over REST](serve.md) (which refreshes the index incrementally before each
    query).

## Supported subset

anarchie implements a practical subset of AQL:

- `SELECT` of leaf paths and the aggregates `COUNT`, `MIN`, `MAX`, `SUM`, `AVG`.
- `FROM ... CONTAINS ...` to anchor and constrain the RM hierarchy.
- `WHERE` with comparisons (`=`, `!=`, `<`, `<=`, `>`, `>=`), `MATCHES`, `LIKE`,
  `EXISTS`, and `AND` / `OR` / `NOT`.
- `ORDER BY`, `LIMIT` and `OFFSET`.
- `$`-parameters (bound with `--param`) and `AS` column aliases.

`LIKE` and `MATCHES` are supported on leaf paths.

!!! warning "Paths are canonical, archetype-node-qualified"
    Paths must be the exact archetype-node-qualified canonical paths, not the
    archetype-relative shorthands you may see in tooling. The systolic magnitude
    of a blood-pressure observation, for instance, is:

    ```
    o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude
    ```

    `ORDER BY` and `WHERE` refer to a column by its path, not by its `AS` alias.

## Examples

Count the Compositions in the store:

```bash
$ anarchie aql "SELECT COUNT(*) FROM COMPOSITION c"
{
  "q": "SELECT COUNT(*) FROM COMPOSITION c",
  "columns": [
    {
      "name": "#count"
    }
  ],
  "rows": [
    [
      1
    ]
  ]
}
```

Project the systolic magnitude, filtering on it with a bound parameter:

```bash
$ anarchie aql "SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic \
    FROM COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2] \
    WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude >= \$min" \
    --param min=120
{
  "q": "SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic FROM COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2] WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude >= $min",
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

Raise the threshold above the stored reading and the row drops out, leaving
`"rows": []`.

## See also

- [anarchie index](index.md) · [anarchie query](query.md) · [anarchie serve](serve.md)
- [Roadmap](../reference/roadmap.md)
