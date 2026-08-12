# anarchie knowledge

Inspect and manage clinical knowledge artefacts and packages. The currently
implemented commands inventory an openEHR CKM mirror checkout, resolve source
policy into a deterministic lock, and explain selection decisions.

## Usage

```bash
anarchie knowledge inventory <checkout>
anarchie knowledge resolve <checkout>
anarchie knowledge status <checkout>
anarchie knowledge why <artefact-id> <checkout>
anarchie --format json knowledge inventory <checkout>
```

## anarchie knowledge inventory

Inventory the `local/` and `remote/` source trees in a CKM mirror checkout. The
command does not modify the checkout or an anarchie deployment.

```bash
$ anarchie knowledge inventory ../CKM-mirror
CKM knowledge inventory
Source revision: 72f5a0828145ddb3b19cbfdafef79ba3bcfcf6f2
Artifacts: 734
Archetypes: 689 (232 published)
Templates: 44
Termsets: 1
...
```

The inventory records:

- relative source path and local/remote provenance;
- Git revision and dirty-checkout state;
- SHA-256 content digest;
- archetype/template identity, RM type, ADL version, lifecycle, revision, and languages;
- raw licence declaration and a normalised SPDX hint where recognised;
- specialisation and explicitly selected archetype hard dependencies;
- symbolic archetype slot constraints;
- duplicate IDs, missing dependencies, major-version mismatches, ambiguous dependencies, and metadata parse limitations.

JSON output contains the complete deterministic artefact and issue lists. It
contains no generation timestamp or checkout-specific absolute path, so the same
source tree produces the same result.

!!! note "Inventory, not installation"
    These commands establish and resolve source evidence. They do not install
    artefacts, compile OET templates, or activate templates in a deployment.

## anarchie knowledge resolve

Every `anarchie init` deployment starts with a minimal `knowledge.toml`. From
any directory inside that deployment, resolve it against an inventoried checkout
and write its deterministic `knowledge.lock`:

```bash
anarchie knowledge resolve ../CKM-mirror
```

The manifest and lock default to the deployment root. Outside a deployment, or
for scripting against another location, supply both explicit paths:

```bash
anarchie knowledge resolve ../CKM-mirror \
  --manifest /path/to/knowledge.toml \
  --lock /path/to/knowledge.lock
```

A minimal manifest selects published, English-declaring, CC-BY-SA-3.0 or
CC-BY-SA-4.0 artefacts from the International `local/` source:

```toml
version = 1

[knowledge]
name = "ckm-international-published"
```

The defaults can be made explicit or changed:

```toml
[source]
origins = ["local"]
allow-dirty-checkout = false

[policy]
allowed-lifecycle-states = ["published"]
allowed-licences = ["CC-BY-SA-3.0", "CC-BY-SA-4.0"]
allow-missing-hard-dependencies = false

[languages]
include = ["en"]

[artefacts]
include = ["openEHR-EHR-OBSERVATION.blood_pressure.v2"]
```

With no `artefacts.include`, every artefact satisfying policy is selected. With
an explicit list, only those roots and their hard-dependency closure are
selected. Dependencies must satisfy the same source, lifecycle, licence, and
language policy.

Resolution refuses dirty checkouts by default. Duplicate IDs, ambiguous or
policy-excluded dependencies, and unavailable requested major versions are
always blocking. Truly missing dependencies are blocking unless
`allow-missing-hard-dependencies = true`; tolerated issues remain recorded in
the JSON resolution report.

On failure, the command reports every blocker and does not create or overwrite
the lock. On success, it atomically writes exact source revision and inventory
digest evidence, artefact paths and checksums, metadata, hard-dependency edges,
and symbolic slot constraints.

## anarchie knowledge status

Compare the deployment manifest and lock with the current source checkout:

```bash
anarchie knowledge status ../CKM-mirror
```

Status is one of:

- `unresolved` - no lock exists yet;
- `current` - manifest SHA-256, source Git revision, and inventory SHA-256 all match the lock;
- `stale` - a lock exists, but any of those three deterministic inputs changed.

The command does not write files. It accepts the same optional `--manifest` and
`--lock` overrides as `resolve`.

## anarchie knowledge why

Run the same resolution without writing a lock and explain all inventory entries
for one artefact ID:

```bash
anarchie knowledge why openEHR-EHR-OBSERVATION.blood_pressure.v2 \
  ../CKM-mirror
```

The explanation distinguishes policy selection, explicit selection,
hard-dependency inclusion, and exclusions by origin, lifecycle, licence,
language, or requested root set. Related resolution issues are included even
when the complete resolution has blockers. Use global `--format json` for the
machine-readable explanation.

## See also

- [anarchie pack](pack.md) · [anarchie template](template.md)
- [Roadmap](../reference/roadmap.md)
