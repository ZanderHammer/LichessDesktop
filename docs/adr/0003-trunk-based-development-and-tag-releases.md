---
status: accepted
date: 2026-11-03
---

# Trunk-based development with tag-triggered releases and generated release notes

## Context and Problem Statement

We need a simple, repeatable release process for an open-source app with cross-platform installers. There should be no hand-written changelog.

## Considered Options

* Trunk-based on `main`, manual semver tag, release notes from git-cliff
* release-please (bot-maintained release PR)
* GitFlow with release branches

## Decision Outcome

Chosen option: "Trunk-based on `main`, manual semver tag, git-cliff notes", because a release is one deliberate action (push a `vX.Y.Z` tag) with no bot PRs or long-lived branches.

* All work lands on `main` in small commits or short-lived PRs. Merge commits are not used (linear history).
  * To update a PR branch with `main`, rebase it (`git pull --rebase origin main` or `git rebase origin/main`) and force-push with `--force-with-lease`. Never merge `main` into a branch: the commit lint (`merge_commit = false` in `committed.toml`) rejects merge commits.
* A release:
  1. bump `version` in `src-tauri/Cargo.toml` (the only version source; `tauri.conf.json` has none)
  2. commit `chore(release): vX.Y.Z`
  3. tag `vX.Y.Z` and push
* `.github/workflows/release.yml` then:
  * checks that the tag matches the Cargo version
  * generates the notes with [git-cliff](https://git-cliff.org) (`--latest --strip header`)
  * builds the installers for all platforms with `tauri-apps/tauri-action`
  * publishes the GitHub Release once every platform succeeds
* There is no `CHANGELOG.md`. GitHub Releases are the changelog.
* The `release` skill (`.claude/skills/release`) documents the steps.

### Consequences

* Good, because releases are explicit and reproducible, and history stays linear.
* Good, because release notes come for free from conventional commits (ADR 0004).
* Bad, because the version bump is a manual step. `git cliff --bumped-version` suggests the next version.
