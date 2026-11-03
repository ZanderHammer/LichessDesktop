# Lichess Desktop

Unofficial lightweight desktop wrapper for lichess.org, built with Tauri v2 in pure Rust (no Node). The app crate is in `src-tauri/` and the Cargo workspace is at the root.

## Commands
- `cargo tauri dev` runs the app. `cargo tauri build` builds the installers.
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` must pass before every commit.

## Rules
- Always look things up (docs.rs, v2.tauri.app, the `tauri-docs` MCP, the `tauri` skill). Never guess APIs, versions or config keys.
- Conventional Commits. Keep commits small: one concern per commit, different changes in different commits.
- No AI/LLM attribution in commits or PRs (no `Co-Authored-By` trailers, no "Generated with" lines).
- Trunk-based on `main`. Releases are made by pushing tags; use the `release` skill. No CHANGELOG file.
- The only version source is `src-tauri/Cargo.toml`.
- Record decisions as ADRs in `docs/adr/` (use the `adr` skill) rather than in this file.
- The web content must stay without IPC access, behind the navigation allowlist (ADR 0005).
