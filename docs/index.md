# anarchie

> *an-archie* - an **archie** without a server. Anarchic, file-first openEHR persistence.

A local-first openEHR Clinical Data Repository (CDR) that uses **flat files as its primary persistence layer** instead of a database server, and **git as its versioning mechanism** instead of rows in Postgres. Written in Rust. No JVM. No Docker. No terminology server.

The wager is simple: **openEHR data is already document-oriented.** A `COMPOSITION` is a self-contained, versioned clinical document, so the most natural way to store it is as a document on disk - one immutable canonical-JSON file per version. The EHR is a directory. A `CONTRIBUTION` is a git commit. The audit trail is the commit graph.

```bash
# scaffold a CDR, create a patient record, commit a composition
anarchie init --system-id anarchie.example.org
EHR=$(anarchie ehr new)
anarchie commit "$EHR" vitals.json -m "Admission observations"
```

[:octicons-arrow-right-24: Full walkthrough](walkthrough/index.md) ·
[:octicons-arrow-right-24: Why anarchie?](why/why-anarchie.md) ·
[:octicons-arrow-right-24: Roadmap](reference/roadmap.md)

---

<div class="grid cards" markdown>

-   :material-file-tree:{ .lg .middle } __The files are the database__

    ---

    Every Composition is one canonical-JSON file on disk. The EHR is a
    directory. A human with `ls`, `cat`, `jq`, and `git log` can read the whole
    store without `anarchie` installed.

    [:octicons-arrow-right-24: On-disk format](reference/on-disk-format.md)

-   :material-source-branch:{ .lg .middle } __Git is the version history__

    ---

    A `CONTRIBUTION` is a git commit carrying the openEHR `AUDIT_DETAILS` as
    commit metadata and trailers. `git log -- <path>` *is* the version history
    of a Composition. Time-travel and diffing come for free.

    [:octicons-arrow-right-24: Versioning and git](reference/versioning-and-git.md)

-   :material-check-decagram:{ .lg .middle } __Canonical and diffable__

    ---

    openEHR defines a canonical JSON serialisation. `anarchie` round-trips the
    Reference Model through it byte-stably, so two equal Compositions produce
    identical files - and a re-commit of unchanged content diffs to nothing.

    [:octicons-arrow-right-24: The Reference Model](walkthrough/reference-model.md)

-   :material-language-rust:{ .lg .middle } __One Rust binary__

    ---

    Validation, storage, AQL, the REST API, and an MCP server all compile into
    a single dependency-light binary. The only runtime dependency is the system
    `git`.

    [:octicons-arrow-right-24: Getting started](walkthrough/getting-started.md)

</div>

---

## Status

`anarchie` is an early but already-working exploration. What works today:

- **Reference Model core** - parse, validate the shape of, and canonically
  re-serialise openEHR Compositions (`anarchie info`, `anarchie canonicalise`).
- **Git-backed store** - `anarchie init`, one git repository per EHR, and
  committing Compositions as Contributions with full version history
  (`anarchie ehr`, `anarchie commit`, `anarchie cat`, `anarchie log`,
  `anarchie diff`).
- **Validation** - native RM + Operational Template validation, wired into
  `commit` so nonconformant data is rejected at the door with a precise openEHR
  path (`anarchie validate`, `anarchie template`).
- **Batteries included** - `anarchie init` seeds an IPS-aligned set of starter
  templates by default; more can be added with `anarchie pack`.
- **AQL query engine** - a SQLite path-extraction index with an AQL-to-SQL
  translator, plus ad-hoc and stored queries (`anarchie index`, `anarchie aql`,
  `anarchie query`).
- **Services** - the openEHR REST API (`anarchie serve`) and a stdio MCP server
  for LLM agents (`anarchie mcp`).
- **Integrity** - `anarchie fsck` re-validates every stored Composition against
  the RM, independent of the index.

Genuinely still ahead: ingesting `.opt` XML from Archetype Designer, terminology
binding validation via `sct`, a FHIR / IPS projection, and prebuilt-binary
distribution. See the [roadmap](reference/roadmap.md).

!!! warning "Not for clinical use"
    `anarchie` is a research and design exploration. It is not a certified or
    production CDR and must not be used with real patient data.
