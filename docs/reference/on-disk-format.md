# On-disk Format

An `anarchie` deployment is a directory tree of plain files. There is no hidden
state - what you see is the whole store.

```text
my-cdr
├── anarchie.toml              # deployment config
├── ehrs                       # one git repository per patient
│   └── <ehr-id>
│       ├── ehr.json           # the EHR object
│       ├── ehr_status
│       │   └── status.json    # the EHR_STATUS object
│       ├── compositions
│       │   └── <object-id>
│       │       └── composition.json   # canonical head version
│       └── contributions
│           └── <id>-contrib.json      # contribution manifest
├── index                      # derived query index (gitignored)
│   └── .gitignore
└── templates                  # registered OPTs
    └── index.json
```

## Key conventions

- **One canonical JSON file per Composition head.** The working-tree file always
  holds the latest version; every earlier version lives in git history. This is
  the *working-tree-holds-head* convention.
- **One git repository per EHR.** A patient record is self-contained and
  portable - clone, back up, or hand over a single record without touching the
  rest of the store.
- **The index is derived and disposable.** `index/` is `.gitignore`d. It is
  regenerable from the canonical files, so it never belongs in version control.
- **Templates live outside the data.** The schema (OPTs) is registered once
  under `templates/`, not copied into every record, so the files stay lean.

## version_uid

Every committed version has a `version_uid` of the form:

```text
<object_id>::<system_id>::<version_tree_id>
```

- `object_id` - the stable identity of the versioned object (a UUID).
- `system_id` - the creating system, from `anarchie.toml` (set at `init`).
- `version_tree_id` - a 1-based counter; `1` for the first version, incrementing
  on each new version of the same object.

## Reading it back

Because everything is canonical JSON in an ordinary git repository, the store is
fully readable with standard tools - `ls`, `cat`, `jq`, `ripgrep`, `git log`,
`git show` - with no `anarchie` binary required. See
[Inspecting the Files](../walkthrough/inspecting-the-files.md).

The authoritative design lives in the
[specs](https://github.com/pacharanero/anarchie/tree/main/specs) in the
repository.
