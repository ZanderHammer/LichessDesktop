---
status: accepted
date: 2026-11-04
---

# Challenge and game-start notifications from the event stream

## Context and Problem Statement

With a connected account ([ADR 0024](0024-connect-a-lichess-account-with-oauth-pkce-and-the-os-keyring.md)), the app can tell the user about incoming challenges and games that start while its window isn't focused (#25). The page's title can't say this. We need to keep `/api/stream/event` open from Rust, and to tell what is news from what the stream only repeats.

Checked against lichess's [API spec](https://github.com/lichess-org/api/blob/master/doc/specs/tags/board/api-stream-event.yaml) and lila's `master` branch on 2026-11-04:

* **Replay.** [`EventStream.scala`](https://github.com/lichess-org/lila/blob/master/modules/api/src/main/EventStream.scala) offers all current games and incoming challenges as ordinary `gameStart` and `challenge` lines when a stream opens. Nothing marks them as old. The first keep-alive, an empty line, comes 7 seconds later, because its timer starts after the replay.
* **Dropped lines.** The server's queue holds 32 lines and drops the oldest, so a replay of more than 32 can be incomplete.
* **Outgoing challenges.** A `challenge` line also reports what the user sends. It has no `direction`, as `EventStream` passes none to the JSON view, so only `destUser` tells them apart. `challenger` is empty for an anonymous challenger, and `destUser` for an open challenge.
* **IDs.** A challenge and the game it becomes share an ID.
* **Limits.** One stream per token: a new one closes the previous one. [`Limiters.scala`](https://github.com/lichess-org/lila/blob/master/modules/web/src/main/Limiters.scala) allows 30 connections per user in 10 minutes. Without a valid token the endpoint answers 401, as measured with curl.
* **Links.** A challenge's `url` is `/game/<id>`. [`Round.watcher`](https://github.com/lichess-org/lila/blob/master/app/controllers/Round.scala) shows the game there, or else the challenge. `fullId` opens the player's own page.
* **`ureq`.** The app's `timeout_global` would cut a body that never ends. [`timeout_per_read`](https://docs.rs/ureq/3.5.1/ureq/config/struct.ConfigBuilder.html#method.timeout_per_read) restarts for each read, which is what a stream needs.
* **Replacing.** On Linux, the notification plugin sends every notification with `replaces_id` 0, so one with the same ID doesn't replace an earlier one.

## Considered Options

* Remember seen IDs, and treat what comes before the first keep-alive of the first connection as replay
* Treat what comes before the first keep-alive of every connection as replay
* Ignore what comes in a fixed time after each connection opens

## Decision Outcome

Chosen option: "Remember seen IDs, and treat what comes before the first keep-alive of the first connection as replay", because it can't repeat a notification and still reports what happened during a dropped connection. The keep-alive is a boundary lila's code guarantees, unlike a time window that a slow network could overrun.

* **News.** Challenges and games are remembered separately, for as long as the stream runs. On the first connection, everything before the first keep-alive is only remembered: the page shows the same things. If that connection drops first, the next one is still replay. After a reconnect, an unseen ID happened during the gap and alerts.
* **Filters.** A challenge alerts only if `destUser` is the user. A game alerts only if `hasMoved` is false, since whoever has moved knows the game. This also keeps a game out that a full queue dropped from an earlier replay. What arrives while the window has focus is remembered and never alerts.
* **Lifetime.** One thread, while the status has a token and `notify_account_events` is on. Every status change and that setting start or stop it. Stopping drops a channel's sender, so a wait ends at once and a read at the next line. A new stream closes the old one on the server anyway.
* **Reconnecting.** 2 seconds, doubling up to 5 minutes, and starting over after a connection that lasted a minute. That is 9 attempts in the first 10 minutes. A 429 waits at least a minute, which [lichess's docs](https://github.com/lichess-org/api/blob/master/doc/specs/lichess-api.yaml) call enough in most cases, and the backoff keeps growing in case it isn't. The read timeout is 21 seconds, three missed keep-alives, so a connection that died during a suspend is noticed.
* **401.** Removes the token and shows the "expired" message, as a failed check does (ADR 0024). The old token's thread can't do this after it was told to stop, so it can't disconnect a newer token.
* **Username.** It comes from the account status. If lichess.org couldn't be reached at startup, the thread asks with the same backoff before it opens the stream. Its answer also turns "unverified" into "connected".
* **Token.** Read from the store once per thread, so a reconnect can't bring up an unlock prompt.
* **Notification.** Goes through the code of [ADR 0022](0022-alerts-per-game-and-clicks-that-open-the-game.md): the same ID, silent, and a click that focuses the window and opens a lichess.org page. Taskbar attention follows its own setting too, since its text is "when a game needs you", which goes beyond what #25 asks.
* **Setting.** A switch in the notification settings, shown only while an account is connected.
* **IPC.** No new command, and the page still has none. The setting is one more typed value of `set_setting`.

### Consequences

* Good, because a reconnect or a suspend repeats nothing and loses little.
* Good, because a dead connection is found within 21 seconds, without a watchdog thread.
* Good, because people who don't connect an account or turn this off have no thread or connection.
* Bad, because a real event in the first 7 seconds after the stream first opens gives no alert. The app has only just started at that point, or the user has just connected.
* Bad, because a game with no move by the user that a full queue dropped from an earlier replay alerts after a reconnect. It takes more than 32 current games and a dropped connection.
* Bad, because a game that started and got the user's first move during a dropped connection gives no alert. The user has seen it.
* Bad, because the seen IDs grow until the stream stops. They are a few bytes per game, so this doesn't matter in practice.
* Bad, because a stopped thread lives on for up to 21 seconds if its connection has gone silent. It sends nothing in that time.
* Bad, because this was tested with lichess's documented examples and a local server, not with a live account on each platform. Linux, macOS and Windows need checking in the running app.
* Bad, because no native speaker has reviewed the three new `desktop.*` strings (ADR 0020).

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
