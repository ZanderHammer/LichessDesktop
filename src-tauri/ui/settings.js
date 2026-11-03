// Settings/about window (#15). Strings, settings and app facts come from
// Rust; every change is saved at once through `set_setting` (ADR 0012).

import { invoke, listen } from "./ipc.js";

let strings = {};

/** The string `id` with lila's `%s` / `%1$s` placeholders filled in. */
function t(id, ...args) {
  let text = strings[id] ?? id;
  args.forEach((arg, i) => {
    text = text.replace(`%${i + 1}$s`, arg).replace("%s", arg);
  });
  return text;
}

async function loadStrings() {
  const view = await invoke("get_strings");
  strings = view.strings;
  document.documentElement.lang = view.lang;
  document.documentElement.dir = view.dir;
  for (const el of document.querySelectorAll("[data-i18n]")) {
    el.textContent = t(el.dataset.i18n);
  }
}

async function loadSettings() {
  const settings = await invoke("get_settings");
  for (const input of document.querySelectorAll("[data-setting]")) {
    input.checked = settings[input.dataset.setting];
  }
  const select = document.getElementById("language");
  const systemName =
    settings.languages.find(([tag]) => tag === settings.systemLanguage)?.[1] ?? settings.systemLanguage;
  select.replaceChildren(
    new Option(t("desktop.systemLanguage", systemName), "system"),
    ...settings.languages.map(([tag, name]) => {
      const option = new Option(name, tag);
      option.lang = tag;
      return option;
    }),
  );
  select.value = settings.language;
}

async function loadAbout() {
  const info = await invoke("get_app_info");
  document.getElementById("version").textContent = t("desktop.version", info.version);
  document.getElementById("link-source").href = info.repository;
  document.getElementById("link-license").href = info.license_url;
}

async function save(key, value) {
  await invoke("set_setting", { change: { key, value } });
}

function bindSettings() {
  for (const input of document.querySelectorAll("[data-setting]")) {
    input.addEventListener("change", async () => {
      try {
        await save(input.dataset.setting, input.checked);
      } catch (err) {
        input.checked = !input.checked;
        console.error(`failed to save ${input.dataset.setting}:`, err);
      }
    });
  }
  const select = document.getElementById("language");
  select.addEventListener("change", async () => {
    try {
      await save("language", select.value);
    } catch (err) {
      console.error("failed to save the language:", err);
      await loadSettings();
    }
  });
}

/** WAI-ARIA tabs: arrow keys, Home and End move between tabs. */
function bindTabs() {
  const tabs = [...document.querySelectorAll('[role="tab"]')];
  const select = (tab) => {
    for (const other of tabs) {
      const selected = other === tab;
      other.setAttribute("aria-selected", String(selected));
      other.tabIndex = selected ? 0 : -1;
      document.getElementById(other.getAttribute("aria-controls")).hidden = !selected;
    }
    tab.focus();
  };
  for (const tab of tabs) {
    tab.addEventListener("click", () => select(tab));
    tab.addEventListener("keydown", (event) => {
      const rtl = document.documentElement.dir === "rtl";
      const i = tabs.indexOf(tab);
      const step = { ArrowRight: rtl ? -1 : 1, ArrowLeft: rtl ? 1 : -1 }[event.key];
      let next;
      if (step) next = tabs[(i + step + tabs.length) % tabs.length];
      else if (event.key === "Home") next = tabs[0];
      else if (event.key === "End") next = tabs[tabs.length - 1];
      if (next) {
        event.preventDefault();
        select(next);
      }
    });
  }
}

async function main() {
  bindTabs();
  bindSettings();
  await loadStrings();
  await Promise.all([loadSettings(), loadAbout()]);
  document.documentElement.classList.remove("loading");
  // The content is in place: Rust shows the window now. (No
  // requestAnimationFrame here: hidden windows don't get animation frames.)
  await invoke("settings_ready");
  await listen("strings-changed", async () => {
    await loadStrings();
    await Promise.all([loadSettings(), loadAbout()]);
  });
}

main().catch((err) => console.error("settings window failed to load:", err));
