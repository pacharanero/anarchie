# Installing anarchie

`anarchie` is a single, dependency-light binary. The only runtime requirement is the system `git`.

!!! warning "Early software"
    anarchie is a research and design exploration - experimental, and not for use with real patient data. See the [roadmap](reference/roadmap.md).

## Quick install (one-liner)

```sh
curl -LsSf https://pacharanero.github.io/anarchie/install.sh | sh
```

This is the recommended way in, and the command is stable. Today it builds anarchie from source with Cargo, so you need a [Rust toolchain](https://rustup.rs); once tagged releases are published it will fetch a prebuilt binary instead, with no toolchain required - the command above will not change. (A PowerShell one-liner for Windows is on the roadmap.)

## With Cargo

If you already have Rust, install straight from the repository - this is the simplest route for the current technical audience:

```sh
cargo install --git https://github.com/pacharanero/anarchie anarchie-cli --locked
```

The package is `anarchie-cli`; it installs a binary called `anarchie`. Once anarchie is published to [crates.io](https://crates.io) this shortens to:

```sh
cargo install anarchie   # planned
```

## From source

```sh
git clone https://github.com/pacharanero/anarchie
cd anarchie
s/install                # cargo install --path crates/anarchie-cli
```

`s/install` passes extra arguments through to `cargo install` (e.g. `s/install --locked`).

## Verify

```sh
anarchie --version
anarchie --help
```

Then jump into the [walkthrough](walkthrough/index.md) - `anarchie init` gives you a CDR stocked with the IPS-aligned starter templates immediately.

## More install channels (roadmap)

anarchie will follow a *bump-on-`main`, CI-does-the-rest* release model: a maintainer bumps the version, and the pipeline builds prebuilt binaries (via [cargo-dist](https://opensource.axo.dev/cargo-dist/)), cuts the GitHub Release, and updates the install channels from a single `SHA256SUMS`. Planned channels:

| Channel | Command | Status |
|---|---|---|
| One-liner | `curl -LsSf …/install.sh \| sh` | available (builds from source); prebuilt binaries **planned** |
| Cargo (git) | `cargo install --git … anarchie-cli` | available |
| Cargo (crates.io) | `cargo install anarchie` | planned |
| cargo-binstall | `cargo binstall anarchie` | planned |
| Homebrew (macOS / Linux) | `brew install pacharanero/tap/anarchie` | planned |
| Windows | `.msi` / `.exe`, Scoop | planned |
| Debian / Ubuntu | `.deb` | planned |
| Fedora / RHEL | `.rpm` | planned |
| macOS | `.dmg` | planned |

The prebuilt one-liner, Homebrew, and Windows installer come together with the first tagged release; the Linux packages and `.dmg` are additive jobs added when the audience needs them. See [the roadmap](reference/roadmap.md) for the full plan.
