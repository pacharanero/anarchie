# anarchie pack

Install and inspect **archetype packs** - named sets of Operational Templates you
can register in one step. A pack is a convenient bundle of the same
[templates](template.md) you would otherwise add one at a time.

## Usage

```bash
anarchie pack add <name|dir>
anarchie pack list
anarchie pack build <source> <archive.tar.zst>
anarchie pack inspect <archive.tar.zst>
anarchie pack verify <archive.tar.zst>
anarchie pack install <archive.tar.zst>
anarchie pack installed
anarchie pack audit
```

## anarchie pack add

Install a pack. The source is either a bundled pack name or a path to a local
directory of `*.opt.json` template files.

Install the bundled IPS-aligned starter set:

```bash
$ anarchie pack add ips-core
Installed 8 template(s) from pack `ips-core`:
  - vital_signs_encounter.v1
  - problem_list.v1
  - adverse_reaction_list.v1
  - medication_list.v1
  - laboratory_result_report.v1
  - immunisation_list.v1
  - procedure_list.v1
  - encounter_note.v1
```

`ips-core` is anarchie's bundled, IPS (International Patient Summary) aligned
starter set - eight starter templates spanning the IPS content sections: vital
signs, problems, allergies, medications, laboratory results, immunisations,
procedures, and an encounter note. That covers all three *required* IPS
sections (problems, allergies, medications) plus the recommended ones.

Install every `*.opt.json` in a local directory by passing its path:

```bash
$ anarchie pack add ./my-templates
Installed 1 template(s) from pack `./my-templates`:
  - vital_signs_encounter.v1
```

In both cases `add` prints `Installed N template(s) from pack <source>:` followed
by the registered template ids. Installing a pack is equivalent to running
[anarchie template add](template.md) for each of its templates.

## anarchie pack list

List the bundled packs available to install:

```bash
$ anarchie pack list
Bundled packs:
  - ips-core
```

## Package archives

Build a reproducible, data-only knowledge package archive from a package source
directory:

```bash
anarchie pack build ./example-package ./example-1.0.0.tar.zst
anarchie pack inspect ./example-1.0.0.tar.zst
anarchie pack verify ./example-1.0.0.tar.zst
```

The source must contain `knowledge-package.toml` with a package name, version,
and `format_version = 1`. Package data is limited to `artefacts/` and
`provenance/`; the builder writes deterministic tar metadata, zstd compression,
and `checksums.sha256` for every data file.

`inspect` validates archive paths, entry types, manifest identity, file-count
and compressed/expanded-size limits without extraction. `verify` additionally
checks every declared SHA-256. Links, unsafe or non-canonical paths, undeclared
paths, duplicate entries, and checksum mismatches are rejected.

`install` runs the same full verification, then writes the package into the
current deployment under `knowledge/packages/sha256-<archive-digest>/`. It
stages the complete verified package before atomically activating that directory;
failure removes the staging directory and leaves existing package content alone.
Installing the same archive again is idempotent.

`installed` lists the deployment's content-addressed package material. `audit`
reverifies the retained archive, then rehashes every installed package file against
its archive checksums. It is read-only and fails if the archive or package material
has been altered.

!!! note "Build and verify only"
    Content-addressed package material is installed and auditable, but it is not
    yet activated for validation and no templates are registered from it.
    Removal, activation, rollback, and coexistence policy remain K8 work because
    contributions must retain the exact template digests they used. `pack add`
    continues to install legacy local template directories and does not accept
    archive files yet.

## Roadmap

Packs are installed from what is bundled with the binary or from a local
directory. The Knowledge Artefacts Manager now provides CKM inventory,
manifest/lock resolution, and reproducible package build/inspection/verification.
Secure extraction and installation, activation, and a networked registry remain
future work.

## See also

- [anarchie template](template.md) · [anarchie validate](validate.md) · [anarchie knowledge](knowledge.md)
- [Roadmap](../reference/roadmap.md)
