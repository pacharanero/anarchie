# anarchie init

Scaffold a new `anarchie` deployment - the directory tree that holds every EHR,
template, and derived index.

## Usage

```bash
anarchie init [path] [--system-id <id>]
```

| Argument / option   | Default           | Description                                           |
| ------------------- | ----------------- | ----------------------------------------------------- |
| `[path]`            | `.`               | Directory to create the deployment in.                |
| `--system-id <id>`  | `anarchie.local`  | The creating-system identity stamped into every `version_uid`. |

## Example

```bash
$ anarchie init --system-id anarchie.example.org
Initialised anarchie deployment at /home/you/my-cdr
  system_id: anarchie.example.org
```

## What it creates

| Path            | What it is                                                    |
| --------------- | ------------------------------------------------------------ |
| `anarchie.toml` | Deployment config: `system_id`, RM version, index settings.  |
| `ehrs/`         | One git repository per patient record.                       |
| `templates/`    | Registered Operational Templates and their index.            |
| `index/`        | The derived query index. Disposable and `.gitignore`d.       |

The `index/` directory is `.gitignore`d because it is a derived view,
regenerable from the canonical files, and never belongs in version control.

## See also

- [Getting Started](../walkthrough/getting-started.md)
- [On-disk Format](../reference/on-disk-format.md)
