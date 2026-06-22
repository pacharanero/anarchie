# Inspecting the Files

The whole point of `anarchie` is that the store is not a black box. After the
previous page you have a real repository on disk. Here is how to read it back
with tools you already have - **no `anarchie` binary required.**

## Walk the tree

```bash
$ tree my-cdr
my-cdr
├── anarchie.toml
├── ehrs
│   └── 1b4e28ba-2fa1-11d2-883f-0016d3cca427
│       ├── ehr.json
│       ├── ehr_status
│       │   └── status.json
│       ├── compositions
│       │   └── 9f1c8a3e-7c2b-4e9a-bd11-2a1f6c0e4d55
│       │       └── composition.json
│       └── contributions
│           ├── 4d8e…-contrib.json
│           └── 7b2a…-contrib.json
├── index
│   └── .gitignore
└── templates
    ├── adverse_reaction_list.v1.opt.json
    ├── attribution.md
    ├── index.json
    ├── problem_list.v1.opt.json
    └── vital_signs_encounter.v1.opt.json
```

Each EHR is a directory. Each Composition is a directory holding the canonical
head file. Each Contribution leaves a manifest. It is all just files.

## Read a Composition with jq

```bash
$ jq '.name.value, .archetype_details.template_id.value' \
    ehrs/*/compositions/*/composition.json
"Blood pressure"
"vital_signs_encounter.v1"
```

Because the files are canonical JSON, `jq`, `ripgrep`, and friends work without
any special knowledge of `anarchie`.

## Read the history with git

The version history *is* the git history:

```bash
$ cd ehrs/1b4e28ba-2fa1-11d2-883f-0016d3cca427
$ git log --oneline -- compositions/9f1c8a3e-*/composition.json
b2c3d4e Corrected diastolic reading
a1b2c3d Admission observations
```

The openEHR audit metadata is right there in the commit:

```bash
$ git log -1 --format='%an <%ae>  %cI%n%b'
Dr Ada Lovelace <ada@example.org>  2026-01-15T09:42:11Z
anarchie-contribution-id: 7b2a…
anarchie-change-type: modification
anarchie-system-id: anarchie.example.org
```

## Reconstruct an old version with git

```bash
$ git show a1b2c3d:compositions/9f1c8a3e-*/composition.json | jq .name.value
"Blood pressure"
```

This is exactly what `anarchie cat <version_uid>` does under the hood. There is
no hidden state: the data lives in files, the history lives in git, and both are
open formats you can audit, back up, and grep forever.

!!! success "The onion model"
    The canonical JSON files at the centre are the source of truth. The git
    history, the (future) AQL index, and the (future) REST API are all
    **derived views**. Delete a derived layer and rebuild it; the patient data is
    untouched because it never lived there.
