# Walkthrough

This walkthrough shows off the feature set that `anarchie` ships **today**, end
to end, using nothing but the `anarchie` binary and ordinary shell tools.

By the end you will have:

- built a brand new flat-file CDR from scratch,
- created an EHR (a patient record) as its own git repository,
- committed a clinical Composition as an audited openEHR `CONTRIBUTION`,
- committed a second version and watched the version history grow,
- reconstructed an older version and diffed two versions,
- registered an Operational Template and watched non-conformant data get
  rejected at commit time,
- and inspected the entire store with `ls`, `cat`, `jq`, and `git log` - no
  `anarchie` required to read it back.

## The pages

<div class="grid cards" markdown>

-   :material-rocket-launch:{ .lg .middle } __[Getting Started](getting-started.md)__

    ---

    Build the binary and scaffold your first repository with `anarchie init`.

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

</div>

!!! tip "Follow along"
    Every command on these pages is real and runnable against the current
    build. Sample output is shown so you can check your results as you go.
