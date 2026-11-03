---
name: release
description: Cut a new Lichess Desktop release. Computes the next semver from conventional commits with git-cliff, bumps src-tauri/Cargo.toml, commits, tags vX.Y.Z on main and pushes, which triggers the release workflow. Use when the user asks to release, tag, publish or ship a version.
---

# Release

Releases are tag-triggered on `main` (ADR 0003). The tag push runs `.github/workflows/release.yml`, which builds all installers and publishes a GitHub Release whose notes git-cliff generates. There is no CHANGELOG file.

## Preconditions
- On `main`, working tree clean, up to date with `origin/main`.
- CI on `main` is green.
- `git-cliff` is installed (`cargo install git-cliff --locked`).

## Steps
1. Preview the notes: `git cliff --unreleased --strip header`. If nothing user-facing changed, stop and tell the user.
2. Get the next version: `git cliff --bumped-version` (prints e.g. `v0.2.0`; the bump rules are in `cliff.toml` under `[bump]`). Confirm it with the user.
3. Set `version = "X.Y.Z"` (no `v`) in `src-tauri/Cargo.toml`, then run `cargo check` so `Cargo.lock` updates.
4. Commit only those two files: `git commit -m "chore(release): vX.Y.Z"`, with no AI attribution trailers.
5. Tag: `git tag -a vX.Y.Z -m "vX.Y.Z"`.
6. **Ask the user before pushing** (this publishes): `git push origin main && git push origin vX.Y.Z`.
7. Watch the run with `gh run watch` and report the release URL from `gh release view vX.Y.Z --web`.

## If the release workflow fails
- Fix forward with a new commit on `main`. Don't move or delete a published tag. If no release was published, delete the draft release and tag, and redo after the fix.
