// Command palette (#16): a search field over a listbox of app actions and
// lichess pages, after the WAI-ARIA combobox pattern. Focus stays in the
// field; ↑/↓ move the selection (`aria-activedescendant`), Enter runs it and
// Esc closes the window. Entries come from Rust (`list_actions`) and run
// there (`run_action`), so the page knows ids and labels only (ADR 0012).

import { score } from "./fuzzy.js";
import { invoke } from "./ipc.js";
import { loadStrings, t } from "./strings.js";

/** Group headings. Product names, so they are not translated. */
const GROUPS = { app: "Lichess Desktop", lichess: "lichess.org" };

const search = document.getElementById("search");
const results = document.getElementById("results");
const empty = document.getElementById("empty");

let entries = [];
/** The entries on screen, in screen order. */
let shown = [];
let active = 0;
let running = false;

/** The entries that match `query`, best first within each group. */
function filter(query) {
  const groups = Object.keys(GROUPS);
  return entries
    .map((entry, index) => ({ entry, index, score: score(query, entry.label) }))
    .filter((match) => match.score !== null)
    .sort(
      (a, b) =>
        groups.indexOf(a.entry.group) - groups.indexOf(b.entry.group) ||
        b.score - a.score ||
        a.index - b.index,
    )
    .map((match) => match.entry);
}

function option(entry, index) {
  const el = document.createElement("div");
  el.className = "palette-option";
  el.id = `option-${index}`;
  el.setAttribute("role", "option");
  el.setAttribute("aria-selected", "false");
  const label = document.createElement("span");
  label.className = "row-text";
  label.textContent = entry.label;
  el.append(label);
  if (entry.keys.length > 0) {
    const shortcut = document.createElement("span");
    shortcut.className = "shortcut";
    for (const cap of entry.keys) {
      const kbd = document.createElement("kbd");
      kbd.textContent = cap;
      shortcut.append(kbd);
    }
    el.append(shortcut);
  }
  // Keep the focus in the search field.
  el.addEventListener("mousedown", (event) => event.preventDefault());
  el.addEventListener("mousemove", () => {
    if (active !== index) select(index);
  });
  el.addEventListener("click", () => run(index));
  return el;
}

function render() {
  const query = search.value;
  shown = filter(query);
  const groups = [];
  let index = 0;
  for (const [group, heading] of Object.entries(GROUPS)) {
    const members = shown.filter((entry) => entry.group === group);
    if (members.length === 0) continue;
    const el = document.createElement("div");
    el.setAttribute("role", "group");
    el.setAttribute("aria-labelledby", `group-${group}`);
    const title = document.createElement("div");
    title.className = "palette-group";
    title.id = `group-${group}`;
    title.textContent = heading;
    el.append(title, ...members.map((entry) => option(entry, index++)));
    groups.push(el);
  }
  results.replaceChildren(...groups);
  results.hidden = shown.length === 0;
  search.setAttribute("aria-expanded", String(shown.length > 0));
  empty.textContent = shown.length === 0 ? t("site.thereAreNoResultsForX", query.trim()) : "";
  select(0);
}

function select(index) {
  document.getElementById(`option-${active}`)?.setAttribute("aria-selected", "false");
  active = index;
  const el = document.getElementById(`option-${index}`);
  if (!el) {
    search.removeAttribute("aria-activedescendant");
    return;
  }
  el.setAttribute("aria-selected", "true");
  search.setAttribute("aria-activedescendant", el.id);
  // The first entry shows its group heading too.
  if (index === 0) results.scrollTop = 0;
  else el.scrollIntoView({ block: "nearest" });
}

async function run(index) {
  const entry = shown[index];
  if (!entry || running) return;
  running = true;
  try {
    await invoke("run_action", { id: entry.id });
  } catch (err) {
    running = false;
    console.error(`failed to run ${entry.id}:`, err);
  }
}

function close() {
  invoke("close_palette").catch((err) => console.error("failed to close the palette:", err));
}

function onKey(event) {
  if (event.isComposing) return;
  const count = shown.length;
  switch (event.key) {
    case "ArrowDown":
      if (count) select((active + 1) % count);
      break;
    case "ArrowUp":
      if (count) select((active - 1 + count) % count);
      break;
    case "PageDown":
      if (count) select(Math.min(active + 5, count - 1));
      break;
    case "PageUp":
      if (count) select(Math.max(active - 5, 0));
      break;
    case "Enter":
      run(active);
      break;
    case "Escape":
      close();
      break;
    // The only control: keep the focus here.
    case "Tab":
      break;
    default:
      // Ctrl/Cmd+K again closes the palette.
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        close();
        break;
      }
      return;
  }
  event.preventDefault();
}

async function main() {
  search.addEventListener("input", render);
  search.addEventListener("keydown", onKey);
  window.addEventListener("focus", () => search.focus());
  await loadStrings();
  entries = await invoke("list_actions");
  render();
  document.documentElement.classList.remove("loading");
  search.focus();
  // The content is in place: Rust shows the window now.
  await invoke("palette_ready");
}

main().catch((err) => console.error("command palette failed to load:", err));
