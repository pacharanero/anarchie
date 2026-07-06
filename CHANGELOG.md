# Changelog

All notable changes to `anarchie` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Releases are grouped from conventional-commit messages by [git-cliff](https://git-cliff.org).

## [unreleased]

### Bug fixes

- **init**: Friendlier "already initialised" message ([3cfab02](https://github.com/pacharanero/anarchie/commit/3cfab02cc0905a3512cb5b60e3fadd82b8c85021))

- **s/install**: --force --locked so reinstall always works ([c9be0e0](https://github.com/pacharanero/anarchie/commit/c9be0e0ebdc26f1d157158ae9f4840e11a0d4e0a))

- **store**: Do not GPG-sign anarchie's machine commits ([67ae701](https://github.com/pacharanero/anarchie/commit/67ae7013ae00803a2b3c73bb63523f5bff6b8c41))

### CI

- **docs**: Artifact-based Pages deploy; dynamic-port s/docs ([4038982](https://github.com/pacharanero/anarchie/commit/40389827dbc14e405ed9cd2264187445391c2694))

### Chores

- House-style first PR - publish perms, agent docs, SECURITY, editorconfig ([399aec9](https://github.com/pacharanero/anarchie/commit/399aec9e7851a901de756bc0cc11382c32450398))

- Add opt-in local git hooks ([e5900ba](https://github.com/pacharanero/anarchie/commit/e5900babb5b5211b496399426301e34c16a83cac))

- **licensing**: CC-BY-SA-4.0 for anarchie's own content ([effaf86](https://github.com/pacharanero/anarchie/commit/effaf860719b680823f603f9ed5591ea078ed852))

- House-style quick wins - REUSE, dependabot, CI, s/ scripts ([b039ba9](https://github.com/pacharanero/anarchie/commit/b039ba921d06e358c21d56e4d85a1c535e9498fe))

### Documentation

- **walkthrough**: Ship examples/vitals.json so the steps actually run ([7d8604e](https://github.com/pacharanero/anarchie/commit/7d8604ec60390617b2674224c5cc1064156366cf))

- **roadmap**: Task checkboxes for status at a glance ([a8f041e](https://github.com/pacharanero/anarchie/commit/a8f041eea205aa870b861839cf7f02b05f018cea))

- **roadmap**: Capture house-style conformance gaps ([e434086](https://github.com/pacharanero/anarchie/commit/e4340869d55bddc92f9fca33e477e4d842d31f3a))

- Trim the roadmap to forward-looking work; reconcile the reader summary ([31e5bf9](https://github.com/pacharanero/anarchie/commit/31e5bf919bb7209d315514620af1fc9393b4fcc4))

- Bring the documentation in line with the implemented CLI ([17425ba](https://github.com/pacharanero/anarchie/commit/17425ba98c682327245b048eea7fd45ec1ec6a67))

### Features

- Add cargo-install path and an interim curl|sh installer ([7f855d4](https://github.com/pacharanero/anarchie/commit/7f855d4694a791b93c95d85a0efccc441a17b8de))

### Refactor

- **cli**: Thin main + cli modules; --format, version, completions ([152c38f](https://github.com/pacharanero/anarchie/commit/152c38f96181a651d56718e63e80bf139bcefa9f))

- Single `anarchie` crate, with CI and crates.io publishing ([c89e6f7](https://github.com/pacharanero/anarchie/commit/c89e6f7c8401d0856f4b7789efbc34d6ba694ae1))

### Tests

- Add CLI integration tests and a coverage job ([6103b5f](https://github.com/pacharanero/anarchie/commit/6103b5fd32f2811a84f4db17d994f69142707927))

### Specs

- Fold in ideas from an earlier ferriehr design note ([8261014](https://github.com/pacharanero/anarchie/commit/82610149f84bf2a728df1c543b030dabbb8e2eee))


