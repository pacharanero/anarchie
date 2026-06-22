# Why anarchie?

A conventional openEHR clinical data repository is a server in front of a
relational database:

```text
   App ──REST──▶ CDR server ──SQL──▶ Postgres ──▶ disk
                 (EHRbase etc.)      (jsonb)
```

`anarchie` collapses the middle:

```text
   App ──REST──▶ anarchie ──▶ canonical JSON files on disk
                              (the EHR *is* the directory tree)
```

The canonical Composition file is the source of truth. The query index, the REST
API, the MCP server - all of it is a **derived, regenerable view**. Delete the
index and rebuild it; the patient data is untouched because it never lived in
the index. This is the same onion model as the sibling
[`sct`](https://github.com/pacharanero/sct) project, which replaced a SNOMED CT
*server* with a single greppable artefact and a handful of derived views.

## Why this fits openEHR especially well

openEHR is unusually suited to flat-file persistence, almost by accident of its
design:

1. **Compositions are already documents.** An openEHR Composition is a complete,
   self-describing tree. It does not need joining to be meaningful, so it maps
   1:1 to a file.
2. **There is a canonical serialisation.** Two systems serialising the same
   Composition produce byte-comparable output. That makes files diffable and
   hashable.
3. **The data is immutable and versioned by design.** openEHR never updates a
   Composition in place; it creates a new version. Immutable versions map
   perfectly onto immutable files and onto git commits.
4. **The contribution/audit model maps onto version control.** A `CONTRIBUTION`
   is an audited set of versions committed together - which is exactly what a
   git commit is.
5. **The schema lives outside the data.** Templates define structure; the
   Reference Model defines the substrate. Neither is stored per-record, so the
   files stay lean.

If you set out to design a clinical data standard that could be stored as flat
files, you would end up with something very close to openEHR.

## The honest hard parts

The `sct` analogy gets stretched because SNOMED CT is read-mostly reference
data, while a CDR is a read-write transactional store. That raises problems
`sct` never had to solve - validation, AQL, concurrency, and scale - and
`anarchie` does not hand-wave them: each now has working code as well as a
design document in the
[specs](https://github.com/pacharanero/anarchie/tree/main/specs), and the
[roadmap](../reference/roadmap.md) tracks what is shipped versus deferred.
