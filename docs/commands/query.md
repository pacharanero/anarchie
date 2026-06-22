# anarchie query

Manage and run **stored (named) AQL queries**. A stored query is versioned as
*data* - a plain `.aql` file kept alongside your templates - not as code, so it
is git-versioned with the rest of the deployment.

## Usage

```bash
anarchie query add <name> <file> [--version X.Y.Z]
anarchie query list
anarchie query run <name> [--version X.Y.Z] [--param NAME=VALUE]...
```

## anarchie query add

Register the AQL text in `<file>` under `<name>` and a semantic version
(default `1.0.0`).

```bash
$ anarchie query add high_systolic high-systolic.aql
Registered query high_systolic/1.0.0
```

Register a new version of the same query by passing `--version`:

```bash
$ anarchie query add high_systolic high-systolic.aql --version 1.1.0
Registered query high_systolic/1.1.0
```

Each query is stored as `queries/<name>/<version>.aql` - sibling to
`templates/`, and equally part of the system of record. Because stored queries
are data, they version, diff and travel exactly like every other file in the
deployment.

## anarchie query list

List the registered stored queries as `name/version`:

```bash
$ anarchie query list
high_systolic/1.0.0
high_systolic/1.1.0
```

## anarchie query run

Run a stored query and print the same openEHR ResultSet JSON as
[anarchie aql](aql.md). With no `--version`, the highest registered version
runs. Bind `$`-parameters with `--param`:

```bash
$ anarchie query run high_systolic --param min=120
{
  "q": "SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic FROM COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2] WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude >= $min\n",
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

Pin a specific version with `--version 1.0.0`.

As with [anarchie aql](aql.md), `query run` reads the [index](index.md) as it
stands - run [anarchie index](index.md) after new commits, or run the query
[over REST](serve.md), which refreshes the index first.

## See also

- [anarchie aql](aql.md) · [anarchie index](index.md) · [anarchie serve](serve.md)
- [anarchie template](template.md)
