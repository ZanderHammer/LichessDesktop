// The only module that talks to Tauri (ADR 0012). Bundled pages import from
// here and never touch `window.__TAURI_INTERNALS__` themselves, because that
// object is internal and may change between Tauri releases.
//
// `withGlobalTauri` stays off: it would add `window.__TAURI__` to every
// webview, lichess.org included (ADR 0008). These functions make the same
// calls as `invoke` and `listen` in @tauri-apps/api 2.12 (`core.js`,
// `event.js`).

const internals = () => {
  const tauri = window.__TAURI_INTERNALS__;
  if (!tauri) {
    throw new Error("Tauri IPC is not available in this page");
  }
  return tauri;
};

/**
 * Calls a Rust command. Only commands that this window's capability allows
 * succeed; the rest are rejected before they run.
 *
 * @param {string} cmd Command name, e.g. "get_settings".
 * @param {Record<string, unknown>} [args] Arguments, in camelCase.
 * @returns {Promise<unknown>}
 */
export function invoke(cmd, args = {}) {
  return internals().invoke(cmd, args);
}

/**
 * Listens to an event emitted from Rust. Needs `core:event:allow-listen` and
 * `core:event:allow-unlisten` in the window's capability.
 *
 * @param {string} event Event name.
 * @param {(event: { event: string, id: number, payload: unknown }) => void} handler
 * @returns {Promise<() => Promise<void>>} Resolves to a function that stops listening.
 */
export async function listen(event, handler) {
  const eventId = await invoke("plugin:event|listen", {
    event,
    target: { kind: "Any" },
    handler: internals().transformCallback(handler),
  });
  return async () => {
    window.__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener(event, eventId);
    await invoke("plugin:event|unlisten", { event, eventId });
  };
}
