# Query Engine (AQL)

AQL - Archetype Query Language - is the standard openEHR query interface. Supporting it is what separates a CDR from a document store. Translating AQL over a tree of flat JSON files into something fast is the central engineering challenge above the storage layer, the equivalent of `sct`'s FTS5/DuckDB layer over its NDJSON.

This is an explicitly **open** area: the MVP subset and the build-vs-borrow boundary are not yet fixed. This document lays out the approach and the decision points.

---

## The two query modes (don't conflate them)

| Mode | Example | Needs an index? |
|---|---|---|
| **Direct retrieval by id** | "get Composition `8849…` version 2" | No - filesystem/git lookup |
| **Population query (AQL)** | "all systolic BP > 140 across all EHRs in 2026" | Yes - scanning every file is too slow |

Direct retrieval is always correct and always fast without any index. AQL is where the work is. The architectural stance: **git/files are the system of record (write model); a derived index is the read model.** This is CQRS, and it means AQL never reads the canonical files directly at query time - it reads the index, which is rebuildable from the files.

---

## Why naive scanning fails

You *could* answer AQL by `jq`-ing every `composition.json` in every repo per query. For a teaching demo with 100 compositions, fine. For anything real, every query becomes an O(all data) full scan. So an index is needed - but it must be **derived and disposable**, never authoritative, so that the "files are the truth" principle holds.

---

## The path-extraction index

openEHR data is queried by **archetype paths** - canonical paths like:

```
/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data/events[at0006]/data/items[at0004]/value/magnitude
```

The core idea (used by EHRbase and others): **flatten each Composition into (path, value) rows** and store them in a relational/columnar engine that SQL can hit fast.

```
Composition JSON
      │  anarchie index
      ▼
path-value rows:
  ehr_id | comp_id | version | archetype_path                          | value | value_type
  9f1c…  | 8849…   | 2       | …blood_pressure…items[at0004]/value/mag | 142   | DV_QUANTITY
```

AQL is then translated into SQL/DuckDB over this table (plus a few companion tables for EHR/composition/template metadata). This is the same move `sct` makes: do the expensive join/flatten *once* at index time, then serve cheap queries.

### Storage engine choice

Mirroring `sct`'s dual offering:

- **SQLite** (with JSON1 + FTS5) - the embedded default. Great for point lookups, archetype-path filters, and text search. One file, zero server.
- **DuckDB / Parquet** - the analytics path. DuckDB's first-class JSON and columnar scans can answer aggregate population queries ("count, mean, percentile across N patients") far faster, and may handle a surprising amount of AQL *natively over JSON* before a bespoke translator is needed.

**Open question:** how far does DuckDB's JSON support get us for free before we must write a real AQL→SQL compiler? Possibly quite far for the read-heavy aggregate cases.

---

## AQL translation pipeline

```
AQL string
   │  parse  (grammar → AST)
   ▼
AQL AST  (SELECT paths, FROM EHR/COMPOSITION contains …, WHERE, ORDER BY)
   │  resolve archetype/template paths → index columns
   ▼
logical plan
   │  emit
   ▼
SQL (SQLite)  or  SQL (DuckDB over path table / Parquet)
   │  execute
   ▼
result set  →  openEHR AQL ResultSet JSON
```

- **Parser**: AQL has a published grammar (the openEHR SDK and Archie both have ANTLR grammars; a Rust `nom`/`pest`/`lalrpop` reimplementation is feasible and keeps the no-runtime promise).
- **CONTAINS**: AQL's `CONTAINS` clause expresses archetype containment hierarchy. This maps to joins/filters on the archetype-path structure in the index.
- **Result shaping**: output is the openEHR AQL ResultSet structure so it is REST-API-conformant.

---

## Suggested MVP subset

Don't boil the ocean. A useful first cut of AQL:

1. `SELECT` of leaf paths (`magnitude`, `value`, `defining_code/code_string`).
2. `FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.xxx]`.
3. `WHERE` with comparison operators on leaf values and `MATCHES` for coded values.
4. `ORDER BY`, `LIMIT`, `OFFSET`.
5. Basic aggregates (`COUNT`) - with `DuckDB` doing the heavy lifting for `AVG`/percentiles later.
6. `$`-prefixed parameters (for example `$ehrUid`), resolved at execution time from the request's `query_parameters` map.

Explicitly deferred: full path predicates, complex `CONTAINS` nesting, most `FUNCTION` calls, temporal `VERSION` queries, terminology subsumption in `WHERE` (delegate to `sct`).

---

## Grammar reference

AQL has a published ANTLR4 grammar (`AqlLexer.g4` / `AqlParser.g4` in [openEHR/specifications-QUERY](https://github.com/openEHR/specifications-QUERY)). `anarchie` reimplements the parser in Rust (`nom` / `pest` / `lalrpop` are all viable and keep the no-runtime promise), targeting the MVP subset first. The shape to implement:

```
query         = SELECT distinct? top? select_exprs FROM from_expr where? order_by? limit?
select_expr   = identified_path (AS alias)? | aggregate_fn "(" identified_path ")" (AS alias)?
from_expr     = class_expr (CONTAINS contains_expr)? | contains_expr (AND | OR) contains_expr
class_expr    = rm_type variable? archetype_predicate? | VERSION variable? version_predicate?
where_expr    = identified_path comparison_op terminal
              | identified_path (LIKE pattern | MATCHES "{" value_list "}")
              | EXISTS identified_path | NOT where_expr | where_expr (AND | OR) where_expr
identified_path = variable ("/" object_path)?
predicate     = "[" (at_code | archetype_id | id_code) ("," name_value)? "]"
order_expr    = identified_path (ASC | DESC)?
limit         = LIMIT integer (OFFSET integer)?
```

## Function catalogue

The full set AQL defines, to be implemented incrementally (the index/DuckDB engine supplies most of these for free; the rest are evaluated in the projection step):

| Category | Functions |
|---|---|
| Aggregate | `COUNT`, `MIN`, `MAX`, `SUM`, `AVG` |
| String | `LENGTH`, `CONTAINS`, `POSITION`, `SUBSTRING`, `CONCAT`, `CONCAT_WS` |
| Numeric | `ABS`, `MOD`, `CEIL`, `FLOOR`, `ROUND` |
| Date/Time | `CURRENT_DATE`, `CURRENT_TIME`, `CURRENT_DATE_TIME` / `NOW`, `CURRENT_TIMEZONE` |
| Terminology | `TERMINOLOGY(service, code, expansion)` - delegated to the terminology backend (`sct`) |

## Ad-hoc and stored queries

openEHR distinguishes **ad-hoc** queries (the AQL text is in the request) from **stored** (named) queries (registered once, executed by name and version). `anarchie` supports both:

- **Ad-hoc** - `anarchie aql "SELECT …"` / `POST /v1/query/aql`. The text is parsed, planned, and executed against the index.
- **Stored** - a query is registered under a name and semantic version (`PUT /v1/definition/query/{name}`) and later run by reference (`GET|POST /v1/query/{name}/{version}`). Stored queries are **data, not code**: they live as files under the deployment (alongside templates), are git-versioned, and are inspectable like everything else. This mirrors the template registry - register once, reference by id.

---

## Index freshness

Because the index is the read model, it must not lag the write model silently:

- **Synchronous** - the commit path updates affected index rows in the same operation. Simple correctness, slightly slower writes.
- **Lazy / dirty-mark** - the commit marks the EHR dirty in `manifest.json`; the index is rebuilt for dirty EHRs on next query or on demand. Faster writes, risk of a query racing a rebuild.

The manifest tracks the last-indexed git commit per EHR, so `anarchie` can always detect drift and, in the worst case, **rebuild the entire index from the file tree** - the ultimate backstop that makes the whole derived-index approach safe.

---

## Relationship to `sct`'s query layer

This is the most direct structural borrow from `sct`:

| `sct` | `anarchie` |
|---|---|
| NDJSON artefact | tree of canonical Composition JSON |
| `sct sqlite` (FTS5) | `anarchie index` (path table in SQLite) |
| `sct parquet` (DuckDB) | DuckDB/Parquet path table for analytics |
| ECL engine over SQLite | AQL engine over the path index |
| `sct lexical` | text search across Compositions |

The lesson taken from `sct`: **flatten once, serve many.** The expensive structural work happens at index time; queries stay cheap; and the index can always be thrown away and rebuilt because the canonical files never depended on it.
