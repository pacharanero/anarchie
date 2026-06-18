# Scaling — The Performance Envelope

Flat files are delightful at small scale and questionable at large scale. Intellectual honesty demands we state where `anarchie` is a good idea and where it stops being one, rather than pretending file-first persistence scales like Postgres. `sct` is honest that it targets local-first single-machine use; `anarchie` should be equally honest.

---

## The fundamental tension

`sct` got an easy ride on scale because SNOMED CT is **bounded read-only reference data**: ~1M concepts, fixed per release, never written at query time. A CDR is the opposite - **unbounded, growing, read-write patient data**. The same "files are the database" idea therefore meets pressures `sct` never felt:

- the file count grows without limit as care continues,
- writes are transactional and concurrent,
- population queries must touch many files/repos.

So the design must lean hard on the **derived index as the read model** (see [query-engine.md](query-engine.md)) and treat the file tree primarily as the durable, auditable write model.

---

## Where the costs actually land

| Dimension | Cheap | Expensive | Mitigation |
|---|---|---|---|
| Get one Composition by id | ✅ filesystem/git read | - | none needed |
| Write one Composition | ✅ one file + one commit | git commit overhead per write | batch contributions; consider commit coalescing |
| Population AQL | - | ❌ scanning files | the SQLite/DuckDB index serves these |
| Many EHRs (repo-per-patient) | ✅ isolation | ❌ cross-repo queries via git | index spans all EHRs; git is not the query path |
| Huge single EHR (decades of data) | - | git history + working tree grow | git is good at this; index handles query |
| Filesystem inode pressure | - | ❌ millions of tiny files | directory sharding; archival |

The recurring answer: **never query through git or raw files at population scale; query through the index.** Git/files are for durability, audit, and single-object access.

---

## Rough envelope (to be validated by benchmarks, not asserted)

These are *design targets / hypotheses*, deliberately conservative, to be replaced with real numbers once there is code:

- **Comfortable**: 10³–10⁴ EHRs, 10⁴–10⁶ compositions total. Single machine, single filesystem. Index in SQLite. This covers a clinic, a registry, a research cohort, a teaching deployment, a personal health record.
- **Workable with care**: up to ~10⁷ compositions, with DuckDB/Parquet for analytics, directory sharding, and possibly archival of cold EHRs. Write throughput becomes the limiting factor (git commit per contribution).
- **Wrong tool**: national-scale, 10⁸+ records, high concurrent write rate, sub-millisecond p99 population queries. That is EHRbase-on-a-cluster territory. `anarchie` should say so plainly rather than degrade silently.

The honest framing: `anarchie` competes with EHRbase the way **SQLite competes with Postgres** - not on raw scale, but on simplicity, embeddability, zero-ops, and inspectability for the very large class of problems that never actually need cluster scale.

---

## Specific scaling techniques (available if/when needed)

1. **Directory sharding** - `ehrs/9f/1c/9f1c…/` by id prefix to keep any one directory's entry count sane on large deployments.
2. **Index, don't scan** - already core; population queries never walk the tree.
3. **DuckDB for analytics** - columnar scans for aggregates over the path table beat row-by-row SQLite.
4. **Commit coalescing** - batch multiple Compositions into one contribution/commit to amortise git overhead.
5. **Cold archival** - inactive EHRs can be `git bundle`d and moved out of the hot set; portability is a built-in feature, not an add-on.
6. **`git gc` / pack** - git's own packing keeps the per-EHR object store compact over a long history.
7. **External blob storage** - DICOM and large binaries live outside Compositions (referenced), so the JSON tree stays small and diffable.

---

## Concurrency limits

- **Different EHRs** write fully in parallel (separate repos, no shared HEAD) - the repo-per-patient topology's biggest scaling win.
- **Same versioned object**, concurrent writes are serialised by the optimistic `If-Match` precondition; losers get `412` and retry. No silent merges in the MVP.
- **Index updates** under heavy concurrent write need care (SQLite single-writer); a queue or per-shard index may be needed at the upper end of the envelope.

---

## When to walk away from files

A useful explicit checklist - if several of these are true, `anarchie` is the wrong choice and a server-backed CDR is right:

- sustained write rate exceeds what one git repo per patient can absorb,
- population queries must be sub-millisecond at p99 over 10⁸+ records,
- you need multi-master concurrent writes to the *same* records with automatic conflict resolution,
- you need fine-grained, audited *read* access control as a hard regulatory requirement,
- horizontal scale-out across many nodes is a day-one requirement.

Stating this protects the project's credibility. `anarchie` is for the (very large) space *below* that line - and most openEHR experimentation, teaching, local development, and small deployments live comfortably there.
