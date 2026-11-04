// UI strings for the bundled pages (ADR 0015). Rust resolves the language and
// sends its strings; pages mark translated text with data attributes.

import { invoke } from "./ipc.js";

let strings = {};

/** The string `id` with lila's `%s` / `%1$s` placeholders filled in. */
export function t(id, ...args) {
  let text = strings[id] ?? id;
  args.forEach((arg, i) => {
    // Functions, as `$&` in a string replacement has a special meaning.
    text = text.replace(`%${i + 1}$s`, () => arg).replace("%s", () => arg);
  });
  return text;
}

/**
 * Fetches the strings of the UI language and applies them: `lang` and `dir`
 * on `<html>`, and the text of every `[data-i18n]`, the placeholder of every
 * `[data-i18n-placeholder]` and the `aria-label` of every `[data-i18n-label]`.
 */
export async function loadStrings() {
  const view = await invoke("get_strings");
  strings = view.strings;
  document.documentElement.lang = view.lang;
  document.documentElement.dir = view.dir;
  for (const el of document.querySelectorAll("[data-i18n]")) {
    el.textContent = t(el.dataset.i18n);
  }
  for (const el of document.querySelectorAll("[data-i18n-placeholder]")) {
    el.placeholder = t(el.dataset.i18nPlaceholder);
  }
  for (const el of document.querySelectorAll("[data-i18n-label]")) {
    el.setAttribute("aria-label", t(el.dataset.i18nLabel));
  }
}
