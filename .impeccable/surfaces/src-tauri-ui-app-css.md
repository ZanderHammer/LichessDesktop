---
version: 1
slug: "src-tauri-ui-app-css"
primary_target: "src-tauri/ui/app.css"
related_targets: ["src-tauri/ui/settings.html","src-tauri/ui/palette.html"]
---

# Bundled app UI (shared stylesheet, settings/about, command palette)

Scope: the shared stylesheet `src-tauri/ui/app.css` and the small bundled windows built on it, settings/about (#15) and the command palette (#16). Mode: Operate.

Audience and job: regular lichess players open a small window for a few seconds, change or run one thing, and close it. Keyboard first, and every control has a screen-reader label.

## Direction contract

THESIS: A native settings pane, played straight. It refuses lichess's look and any "own world"; the visitor should know where everything is before noticing a style.

OWN-WORLD: Restrained. A neutral window ground with a second neutral layer for grouped list boxes (12px radius, 1px border, no shadow). Rows are 48px with the label on the left and the control on the right. Pill switches, a segmented tab control, the system UI font at three sizes and two weights. One accent (`--accent`, the OS accent fed by Rust, with a blue fallback) for switches, focus, selection and the primary button only. Follows OS light/dark, forced-colors and reduced-motion.

STORY: Open, see a few grouped rows, flip a switch, close. Changes apply at once; nothing asks to be saved.

FIRST VIEWPORT: A segmented tab control (Settings | About) centred at the top. Under it, a section heading and a rounded list box of rows (label left, switch or select right), then a second heading and box. Everything fits without scrolling at 640×520. The palette is one search field above a dense list, with each item's shortcut right-aligned.

FORM: canon (native OS settings), chosen by the user over the roll; quality bar GNOME Settings and Windows 11 Settings. seed:8672ffadf0c86aca

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Open decisions
- Reading the OS accent from Rust (GNOME/portal `accent-color`, Windows `UISettings`, macOS `controlAccentColor`) is a follow-up. Until then the fallback blue applies.
