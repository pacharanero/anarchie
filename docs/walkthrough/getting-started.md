# Getting Started

## Build the binary

`anarchie` is a Rust workspace. With a recent stable toolchain (1.80+):

```bash
git clone https://github.com/pacharanero/anarchie
cd anarchie
cargo build --release
```

The binary lands at `target/release/anarchie`. For the rest of this walkthrough
we assume it is on your `PATH`:

```bash
export PATH="$PWD/target/release:$PATH"
anarchie --help
```

The only runtime dependency is the system `git` binary - `anarchie` shells out
to it rather than bundling a git library, so the store is an ordinary git
repository you can inspect with the git you already have.

## Scaffold a repository

`anarchie init` creates a new deployment: the directory tree that holds every
EHR, template, and derived index.

```bash
mkdir my-cdr && cd my-cdr
anarchie init --system-id anarchie.example.org
```

```text
Initialised anarchie deployment at /home/you/my-cdr
  system_id: anarchie.example.org
```

The `--system-id` is the identity of *this* creating system. It is stamped into
every `version_uid` so that versions created here are globally distinguishable
from versions created by any other openEHR system.

Look at what was created:

```bash
$ ls -A
anarchie.toml  ehrs  index  templates
```

| Path                 | What it is                                                     |
| -------------------- | ------------------------------------------------------------- |
| `anarchie.toml`      | Deployment config: `system_id`, RM version, index settings.   |
| `ehrs/`              | One git repository per patient record. Empty for now.         |
| `templates/`         | Registered Operational Templates and their index.             |
| `index/`             | The derived query index. Disposable and `.gitignore`d.        |

That `index/` is `.gitignore`d on purpose: it is a **derived view**, regenerable
from the canonical files, so it never belongs in version control.

[:octicons-arrow-right-24: Next: The Reference Model](reference-model.md)
