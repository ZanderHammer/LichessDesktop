---
status: accepted
date: 2026-11-03
---

# Conventional Commits and Semantic Versioning

## Context and Problem Statement

Release notes are generated from commit history (ADR 0003), so commits must be machine-readable and meaningful.

## Considered Options

* Conventional Commits + SemVer, enforced in CI
* Free-form commit messages

## Decision Outcome

Chosen option: "Conventional Commits + SemVer, enforced in CI".

* Commit format: [Conventional Commits 1.0](https://www.conventionalcommits.org/en/v1.0.0/), using the types `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `style`, `chore`, `ci`, `build` and `revert`.
* Commits are small and focused: one concern per commit, and different changes go in different commits.
* Commits and PRs carry no AI/LLM attribution (no `Co-Authored-By` trailers or "generated with" lines).
* Versions follow [SemVer 2.0](https://semver.org). `feat` bumps minor, `fix`/`perf` bump patch, and `!`/`BREAKING CHANGE` bumps major (minor while in `0.x`).
* CI lints commit messages with [crate-ci/committed](https://github.com/crate-ci/committed) (config: `committed.toml`).

### Consequences

* Good, because the release notes and the next version can be derived automatically.
* Bad, because contributors must learn the format, though CI points out mistakes.
