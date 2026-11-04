---
status: accepted
date: 2026-11-04
---

# Connect a Lichess account with OAuth PKCE and the OS keyring

## Context and Problem Statement

Account features such as event stream notifications need a Lichess API token (#24). The user is already logged in in the main window, which must keep its navigation allowlist and stay without IPC ([ADR 0005](0005-remote-content-security.md), [ADR 0012](0012-scoped-ipc-for-bundled-app-windows.md)). We need a way to get a token, a place to keep it, and a redirect that never leaves the app.

Checked against lila's `master` branch on 2026-11-04 and lichess.org itself:

* **`lichess-desktop://` is rejected.** [`RedirectUri.from`](https://github.com/lichess-org/lila/blob/master/modules/oauth/src/main/Protocol.scala) only takes `http`, `https`, `ionic`, `capacitor`, a few legacy app schemes, or a scheme that contains a dot. `GET /oauth` answers 400 ("exotic redirect_uri scheme not allowed") for `lichess-desktop://oauth/callback`, and 303 to the login page for `de.itshammer.lichessdesktop://oauth/callback`.
* **Public clients need no registration.** `client_id` is free-form ("choose any") and only S256 is accepted.
* **The redirect is a 303 after a form POST.** A CSP with `form-action` could block it. Lila's [CSP](https://github.com/lichess-org/lila/blob/master/modules/web/src/main/ContentSecurityPolicy.scala) has no such directive, and the `<meta>` policy of `/login` confirms that. We can't fetch the authorize page itself without a login, but it renders through the same code.
* **The handler sees the redirect.** Measured in WebKitGTK 2.54.2 with a local form POST and a 303 to the custom scheme, `decide-policy` receives it as a redirect, so `on_navigation` can cancel it. With `form-action http: https:` or `form-action 'self'` in the page, the redirect is blocked before `decide-policy` runs.
* **Status codes.** With an invalid token, `GET /api/account` and `DELETE /api/token` answer 401. An invalid code at `POST /api/token` answers 400 `invalid_grant`. Lila asks clients not to poll `/api/account`.
* **`keyring` 4 is not meant for us.** [Its docs](https://docs.rs/keyring/4.2.0/keyring/) tell applications that pick their own stores to avoid it and to use [`keyring-core`](https://docs.rs/keyring-core/1.0.1/keyring_core/) with the store crates they need, from the same project.

## Considered Options

* Custom reverse-domain scheme, caught in `on_navigation`
* Loopback `http://127.0.0.1:<port>` with a listener in Rust
* Register `lichess-desktop` as a scheme with lichess

## Decision Outcome

Chosen option: "Custom reverse-domain scheme, caught in `on_navigation`", because lichess accepts it as it is. It keeps what #24 wanted from the custom scheme: no socket to open, and nothing outside the app involved. A loopback listener would add a server and a port for no gain, and asking lichess to add a scheme isn't needed when lila's error message names this format.

* **Scope.** `challenge:read` only.
* **Flow.** "Connect" in Settings first checks that the credential store works, so nobody authorizes for nothing. It then opens `https://lichess.org/oauth` in the main window and closes the settings window, which would cover the page. The callback sends the main window back to where it was and opens Settings again.
* **Callback.** `de.itshammer.lichessdesktop://oauth/callback`, the app identifier. It is not registered with the OS, unlike the deep link scheme of [ADR 0013](0013-lichess-desktop-deep-link-scheme.md), so other programs can't send one in. Both navigation handlers of the main window catch it before the allowlist, so it can't reach the system browser. A callback whose `state` doesn't match the running attempt is ignored and doesn't end the attempt, as any page or frame in the window could navigate there.
* **Token storage.** Only in the OS credential store: Secret Service on Linux, Keychain on macOS, Credential Manager on Windows, through `keyring-core`. There is no fallback. If the store can't be used, Settings says "can't connect securely", and a token that can't be saved is revoked at once.
* **Opt-in.** A plain `account_connected` setting says there is a token to look for, so people who never connect never touch the store or see an unlock prompt. It holds no secret.
* **Checks.** The token is checked at startup and when Settings opens, at most every 5 minutes. A 401 removes it and shows a message. Any other failure, such as being offline, keeps it and says lichess.org can't be reached.
* **Disconnect.** `DELETE /api/token`, then removal from the store. If lichess.org can't be reached, the token is still removed here and Settings says how to revoke it on lichess.org.
* **IPC.** Four new commands for the settings window only: `get_account`, `connect_account`, `cancel_connect_account` and `disconnect_account`. They take no arguments, and none returns the token. ADR 0005 and ADR 0012 still hold: the page in the main window has no IPC.

### Consequences

* Good, because the token is never on disk in the clear and never reaches a page.
* Good, because a forged or replayed callback does nothing, and an intercepted code is useless without the verifier.
* Good, because nothing changes for people who don't connect.
* Bad, because the flow breaks if lichess adds `form-action` to the authorize page: the click would then do nothing. A loopback address would pass `form-action http:`, so that is the way out.
* Bad, because the main window leaves the current page during the flow. A game keeps running on the server while the user authorizes.
* Bad, because only Linux with KWallet was measured. The redirect handling in WebView2 and WKWebView, and the Windows and macOS stores, need checking in the running app.
* Bad, because lichess shows the page as "Not owned or operated by lichess.org", with the scheme as the origin. That is accurate.
* Bad, because the token lasts until it expires or is revoked, about a year, and there is no refresh token. The user then connects again.

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
