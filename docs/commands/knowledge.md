# anarchie knowledge

Inspect and manage clinical knowledge artefacts and packages. The currently
implemented command is a deterministic, read-only inventory of an openEHR CKM
mirror checkout.

## Usage

```bash
anarchie knowledge inventory <checkout>
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
    This command establishes the evidence needed for `knowledge.toml` and
    `knowledge.lock`. It does not yet resolve package policy, install artefacts,
    compile OET templates, or activate templates in a deployment.

## See also

- [anarchie pack](pack.md) · [anarchie template](template.md)
- [Roadmap](../reference/roadmap.md)
