---
status: accepted
date: 2026-11-04
---

# Alert per game, and a notification click opens that game

## Context and Problem Statement

[ADR 0021](0021-native-notifications-from-document-title-changes.md) sends notifications from title changes and leaves two things to #23: what counts as one state change, and what a click on a notification does. #23 says a click may only go to the game if the plugin really reports clicks on desktop. Otherwise it only raises the app, so we don't navigate behind the user's back.

Findings:

* [`ui/round/src/title.ts`](https://github.com/lichess-org/lila/blob/bdd824062f/ui/round/src/title.ts) sets the title whenever the round page updates it, including when the page loads. So a reload goes `Your turn` → the loading page's title → `Your turn`, and opening a finished game goes straight to `Game Over`. Neither is a change of the game's state.
* [`tauri-plugin-notification`](https://docs.rs/tauri-plugin-notification/2.8.0/) 2.8 has `Notification::on_action` on desktop. From reading its `desktop.rs` (not tried on each platform):
  * Linux: it adds the spec's `default` action and reports `ActionInvoked` as `tap`, for every server that supports actions.
  * Windows and macOS: a thread waits on `notify-rust`'s `wait_for_response` for each notification and reports a click as `tap`.
  * Windows: "a click is only reported while the toast is on screen, not once it moved to the Action Center."
  * "Notifications shown before the first handler or listener existed report nothing."
* The reported action carries the notification's `extra` payload, so a click can name its game without the app storing anything.

## Considered Options

* Track the state per game URL, and let a click focus the window and open the game
* Track one global state, and let a click focus the window and open the game
* Track the state per game URL, and let a click only focus the window

## Decision Outcome

Chosen option: "Track the state per game URL, and let a click focus the window and open the game", because it gives no alert for a reload or for an old game, and the plugin reports clicks on all three platforms, which is the condition #23 set.

* **State.** The detector keeps the last game (its URL without the fragment) and its state. Titles that aren't a turn state don't reset it. The state is also recorded while the window is focused, so leaving the window later doesn't alert about it.
* **Game over.** A game first seen as over gives no alert, because it ended before the app looked at it. Only a game seen running can give a game-over alert.
* **Click.** The notification carries its game's URL. A click focuses the main window and navigates to that URL, but only if it is a lichess.org page and the window isn't showing it already, since navigating would reload it. The user asked for this by clicking, so it also applies if they have moved on to another page. On Wayland, KWin refuses the focus request of a window that is open but not focused, so the window is hidden and shown anew, which does bring it to the front.
* **Tray badge.** While the page says it's the user's turn and the window isn't focused, the tray icon flashes a dot painted at runtime on the normal icon, once per second. A thread runs only during the flashing, so an idle app has no timer. It's a lasting state rather than an alert, so a turn that began while focused shows once the user leaves. It's passive and has no setting of its own.
* **Attention.** Taskbar attention uses `UserAttentionType::Informational`, which doesn't keep bouncing the Dock. It has its own setting and fires for both alerts. Focusing the window clears the request.

### Consequences

* Good, because a reload or an old game gives no alert.
* Good, because a click can't leave lichess.org, whatever the payload says.
* Good, because the tray still shows whose turn it is after a notification is gone.
* Bad, because a Windows toast that moved to the Action Center does nothing when clicked.
* Bad, because a Linux notification server without actions shows the notification, but a click does nothing.
* Bad, because going from game A to game B and back while unfocused counts as a change, since only the last game is kept. Navigating needs the window in practice, so this should be rare.
* Bad, because each shown notification holds a thread on Windows and macOS until it goes away. That is how the plugin reports clicks.

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
