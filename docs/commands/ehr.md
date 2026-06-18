# anarchie ehr

Manage EHRs. Each EHR is one git repository - a self-contained, portable patient
record.

## anarchie ehr new

Create a new, empty EHR and print its id.

```bash
anarchie ehr new [--committer <name>] [--email <email>]
```

| Option              | Default               | Description                       |
| ------------------- | --------------------- | --------------------------------- |
| `--committer <name>`| `anarchie`            | Committer name for the creation audit. |
| `--email <email>`   | `anarchie@localhost`  | Committer email for the creation audit. |

```bash
$ anarchie ehr new --committer "Dr A. Clinician" --email a.clinician@example.org
1b4e28ba-2fa1-11d2-883f-0016d3cca427
```

Behind the id, `anarchie` creates `ehrs/<id>/`, runs `git init`, writes the
`EHR` and `EHR_STATUS` objects, and makes the first (creation) commit.

## anarchie ehr list

List the EHR ids in the deployment.

```bash
$ anarchie ehr list
1b4e28ba-2fa1-11d2-883f-0016d3cca427
```

## See also

- [The Git-backed Store](../walkthrough/the-store.md)
- [anarchie commit](commit.md)
