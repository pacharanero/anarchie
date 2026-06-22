# anarchie pack

Install and inspect **archetype packs** - named sets of Operational Templates you
can register in one step. A pack is a convenient bundle of the same
[templates](template.md) you would otherwise add one at a time.

## Usage

```bash
anarchie pack add <name|dir>
anarchie pack list
```

## anarchie pack add

Install a pack. The source is either a bundled pack name or a path to a local
directory of `*.opt.json` template files.

Install the bundled IPS-aligned starter set:

```bash
$ anarchie pack add ips-core
Installed 4 template(s) from pack `ips-core`:
  - vital_signs_encounter.v1
  - problem_list.v1
  - adverse_reaction_list.v1
  - medication_list.v1
```

`ips-core` is anarchie's bundled, IPS (International Patient Summary) aligned
starter set - currently four starter templates: `vital_signs_encounter.v1`,
`problem_list.v1`, `adverse_reaction_list.v1`, and `medication_list.v1`
(the IPS Medication Summary). The remaining Tier-1 IPS sections - laboratory
results, immunisations, procedures, and an encounter note - are still to come.

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

## Roadmap

Packs are installed from what is bundled with the binary or from a local
directory. A networked registry - integration with **kam**, a Knowledge
Artefacts Package Manager, for fetching and resolving published packs - is
planned future work.

## See also

- [anarchie template](template.md) · [anarchie validate](validate.md)
- [Roadmap](../reference/roadmap.md)
