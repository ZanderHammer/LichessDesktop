---
status: accepted
date: 2026-11-04
---

# Open only web and mail links outside the app

## Context and Problem Statement

ADR 0005 opens every navigation the allowlist refuses in the system browser, through `tauri-plugin-opener`. From Rust, [`open_url`](https://docs.rs/tauri-plugin-opener/2.7.2/tauri_plugin_opener/struct.Opener.html) has no scope and passes the address to the system's default handler (`xdg-open` on Linux). So a page, or a frame it embeds, could send a `file:` address or a scheme such as `smb:` or `ms-msdt:` to the system with no click and no prompt. A browser would block or ask first. This narrows ADR 0005's "any other navigation"; the rest of that ADR stands.

## Considered Options

* Allow `https`, `http` and `mailto`, and refuse the rest
* Ask the user before opening any other scheme
* Keep opening everything

## Decision Outcome

Chosen option: "Allow `https`, `http` and `mailto`", because lichess links to the web and to mail addresses, and a prompt would be new UI for links nobody needs. `open_in_browser` checks `navigation::may_open_externally`, so every window goes through the same rule.

### Consequences

* Good, because a compromised page can't start other programs through the app.
* Bad, because a link with another scheme, such as `tel:`, does nothing. It only leaves a line on stderr.
