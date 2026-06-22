# Walkthrough

This walkthrough shows off the feature set that `anarchie` ships **today**, end to end, using nothing but the `anarchie` binary and ordinary shell tools. It doubles as a demo script: each page is a self-contained act in the story of a server-less, file-first, git-native openEHR CDR.

By the end you will have:

- built a brand new flat-file CDR from scratch - already stocked with IPS-aligned starter templates,
- created an EHR (a patient record) as its own git repository,
- committed a clinical Composition as an audited openEHR `CONTRIBUTION`, with your commit message becoming the audit trail,
- committed a second version and watched the version history grow,
- reconstructed an older version and diffed two versions,
- watched non-conformant data get rejected at the door with a precise openEHR path to the breach,
- queried the whole store with AQL over a derived, disposable index,
- served the same data over the **openEHR REST API**,
- and exposed it to an **LLM agent over MCP** - all from one binary whose only runtime dependency is `git`.

## The pages

<div class="grid cards" markdown>

-   :material-rocket-launch:{ .lg .middle } __[Getting Started](getting-started.md)__

    ---

    Build the binary and scaffold your first repository with `anarchie init` -
    batteries included.

-   :material-file-document-check:{ .lg .middle } __[The Reference Model](reference-model.md)__

    ---

    Inspect and canonicalise a Composition with `anarchie info` and
    `anarchie canonicalise`.

-   :material-source-branch:{ .lg .middle } __[The Git-backed Store](the-store.md)__

    ---

    Create an EHR and commit Compositions. See versions, history, and diffs.

-   :material-folder-eye:{ .lg .middle } __[Inspecting the Files](inspecting-the-files.md)__

    ---

    Read the whole store with plain Unix tools, exactly as designed.

-   :material-shield-check:{ .lg .middle } __[Validation and Templates](validation-and-templates.md)__

    ---

    Register a template and let `anarchie` reject invalid data at the door.

-   :material-database-search:{ .lg .middle } __[Querying with AQL](querying-with-aql.md)__

    ---

    Build the derived index and answer population queries with `anarchie aql`
    and stored queries.

-   :material-api:{ .lg .middle } __[The REST API](the-rest-api.md)__

    ---

    Serve the same store over the openEHR REST API with `anarchie serve` -
    ETags, `If-Match`, and `422`s included.

-   :material-robot-happy:{ .lg .middle } __[The MCP Server](the-mcp-server.md)__

    ---

    Expose the CDR to LLM agents with `anarchie mcp`, where validation becomes a
    self-correction loop.

</div>

!!! tip "Follow along"
    Every command on these pages is real and runnable against the current
    build. Sample output is shown so you can check your results as you go.
