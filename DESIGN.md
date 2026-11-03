---
name: Lichess Desktop
description: Bundled app windows (settings/about, command palette) styled as a native OS settings pane
colors:
  accent: "#1c71d8"
  accent-fg: "#ffffff"
  accent-text: "#1a5fb4"
  window-bg: "#f6f5f4"
  surface: "#ffffff"
  surface-hover: "#f1f0ef"
  surface-active: "#e8e7e6"
  ink: "#1e1e20"
  ink-muted: "#5e5c64"
  border: "#deddda"
  control-border: "#c0bfbc"
  switch-off: "#87858b"
  danger: "#c01c28"
  dark-window-bg: "#222226"
  dark-surface: "#2e2e32"
  dark-ink: "#f6f5f4"
  dark-ink-muted: "#c0bfc4"
  dark-border: "#3d3d42"
  dark-accent-text: "#99c1f1"
  dark-focus: "#78aeed"
typography:
  title:
    fontFamily: "system-ui, -apple-system, Segoe UI, Cantarell, Ubuntu, sans-serif"
    fontSize: "1rem"
    fontWeight: 700
    lineHeight: 1.4
  body:
    fontFamily: "system-ui, -apple-system, Segoe UI, Cantarell, Ubuntu, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.4
  label:
    fontFamily: "system-ui, -apple-system, Segoe UI, Cantarell, Ubuntu, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 400
    lineHeight: 1.4
rounded:
  link: "2px"
  key: "4px"
  control: "6px"
  option: "8px"
  list: "12px"
  pill: "50%"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "24px"
components:
  list-box:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.list}"
  row:
    height: "48px"
    padding: "8px 16px"
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-fg}"
    rounded: "{rounded.control}"
    height: "32px"
    padding: "0 12px"
  button:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
    height: "32px"
    padding: "0 12px"
  switch-on:
    backgroundColor: "{colors.accent}"
    width: "44px"
    height: "24px"
  switch-off:
    backgroundColor: "{colors.switch-off}"
    width: "44px"
    height: "24px"
  palette-option:
    rounded: "{rounded.option}"
    height: "36px"
    padding: "8px 12px"
---

# Design System: Lichess Desktop

The tokens live in `src-tauri/ui/app.css`. This file describes how to use them. When they disagree, the stylesheet wins.

## Overview

**Creative North Star: "The Native Settings Pane"**

The app's own windows should look like they belong to the operating system, not to lichess and not to a brand. The user chose the category standard on purpose. The quality bar is GNOME Settings (libadwaita) and Windows 11 Settings. A window is opened for a few seconds to change or run one thing, so everything has to be where a desktop user expects it.

Density is native: 48px rows in rounded list boxes, label on the left, control on the right. One accent colour (the OS accent, for now a native blue) marks what is on, focused or selected, and nothing else. Surfaces are flat. The theme follows the OS (light/dark), and the windows honour forced colours, reduced motion and right-to-left languages.

**Key Characteristics:**
- System UI font only, at three sizes and two weights
- Grouped list boxes with hairline row dividers and no shadows
- One accent, used only for state (on, focus, selection, primary action)
- Logical properties throughout, so RTL works without overrides
- Every control is a native element (checkbox, select, button) styled lightly

## Colors

Restrained neutrals in the libadwaita family, plus one accent that stands in for the OS accent.

### Primary
- **OS Accent** (`accent`): the "on" track of switches, the primary button, the palette selection tint and the focus ring in light mode. Rust may replace `--accent`/`--accent-fg` on `<html>` with the real OS accent. CSS `AccentColor` can't do this, because webviews return a fixed blue.
- **Accent Text** (`accent-text`, dark `dark-accent-text`): links. It is a separate token so that link text stays above 4.5:1 in both themes.

### Neutral
- **Window Ground** (`window-bg`, dark `dark-window-bg`): the window background behind list boxes.
- **List Surface** (`surface`, dark `dark-surface`): list boxes, selected tab and the palette.
- **Ink** (`ink`) and **Muted Ink** (`ink-muted`): primary text, and subtitles, values and key caps. Both pass 4.5:1 in both themes.
- **Hairline** (`border`): list box outlines and row dividers.
- **Off Track** (`switch-off`, dark `#8b8a91`): the track of an off switch, kept above 3:1 against the surface.
- **Danger** (`danger`): destructive button text only.

### Named Rules
**The State-Only Accent Rule.** The accent marks on, focused, selected or primary, and is never decoration. If a screen shows accent on something that isn't one of those, it is wrong.

**The Fed Accent Rule.** Never hard-code the accent in a component. Use `var(--accent)`, `var(--accent-fg)` and `var(--focus-color)` so the OS colour can flow in from Rust.

## Typography

**Body Font:** the system UI font (`system-ui`, then -apple-system, Segoe UI, Cantarell, Ubuntu, sans-serif)

**Character:** the platform's own voice. Hierarchy comes from weight and grouping, not from a second face.

### Hierarchy
- **Title** (700, 1rem / 16px, 1.4): window headings and the palette search text.
- **Body** (400, 0.875rem / 14px, 1.4): row titles, controls, tabs (600) and section titles (700).
- **Label** (400, 0.8125rem / 13px, 1.4): row subtitles, section notes, palette group labels (600) and key caps (600).

Figures are tabular everywhere (`font-variant-numeric: tabular-nums`), so versions and numbers line up.

### Named Rules
**The One Voice Rule.** One family, three sizes. Hierarchy never adds a display face.

## Layout

Windows are small and fixed-purpose (settings about 640×520, the palette about 600×320). A window has a centred header (the segmented tabs) and a scrolling content area with 24px side padding. Sections stack with 24px between them; a section title sits 8px above its list box. Spacing uses the 4/8/12/16/24px scale. Everything uses logical properties (`inline`/`block`), so RTL mirrors automatically, while a shortcut keeps its LTR key order.

## Elevation & Depth

Flat. List boxes have a 1px hairline and no shadow, and depth comes from tonal layering: the window ground, the list surface, then the hover and active tints. The only shadows are the small lift on the selected segmented tab (`0 1px 2px rgb(0 0 0 / 0.12), 0 1px 1px rgb(0 0 0 / 0.06)`) and the switch thumb (`0 1px 2px rgb(0 0 0 / 0.25)`). They imitate a native control's physical lift.

### Named Rules
**The Flat Box Rule.** A list box never gets a shadow. Its border is its elevation.

## Shapes

Rounded but not soft: list boxes and the switch track 12px, palette options and the tab track 8px, tabs and controls 6px, key caps 4px, focus around inline links 2px. The switch thumb is a circle. Corners are never mixed within one component.

## Components

### Buttons
- **Shape:** gently rounded (6px), 32px tall, 12px side padding, 600 weight.
- **Primary:** accent background with accent-fg text, for the one main action in a view (for example "Open release page").
- **Default:** control background with a control-border outline. Hover uses the hover tint, active uses the active tint.
- **Destructive:** default button with danger text.

### Switch
- A native `<input type="checkbox" role="switch">`, 44×24, with an 18px white thumb. On is the accent track, with the thumb moved to the inline end. The thumb mirrors in RTL. In forced colours the track uses ButtonText/Highlight with a 1px border.

### Cards / Containers (list box and rows)
- **Corner Style:** 12px, with content clipped.
- **Background:** list surface with a hairline border.
- **Rows:** 48px minimum height, 8px 16px padding, hairline dividers between rows. Each row has a title and an optional subtitle on the inline start, and a control or value on the inline end. A row that is a whole control (`label`, `a` or `button`) gets hover and active tints. Link rows end with a 16px muted icon: a chevron for in-app pages, an external-link icon for links that open the browser.

### Inputs / Fields
- **Select:** native select, 32px tall, 6px radius, control background, limited to 60% of the row width.
- **Palette search:** borderless, full width, 48px tall, title size. A hairline sits underneath, replaced by a 2px focus-colour line when focused.

### Navigation
- **Segmented tabs:** `role="tablist"` in a pill track tinted 8% ink. The selected tab sits on the list surface with the tab lift. Unselected tabs are muted ink, ink on hover.

### Command palette
- A search field over a `role="listbox"`. Options are 36px rows with 8px radius. The selected option gets an accent tint at 16% over the surface. Group labels are muted 13px/600. Shortcuts sit at the inline end as key caps inside one `.shortcut` group with 4px gaps.

### Focus
- Every focusable control shows `--focus-ring`: a 2px surface gap and a 2px ring in `--focus-color` (the accent in light, #78aeed in dark). Row links use a 2px inset ring instead. Focus is never removed without a replacement.

## Do's and Don'ts

### Do:
- **Do** build every view from list boxes and rows: label on the inline start, control on the inline end, 48px rows.
- **Do** use native elements (checkbox with `role="switch"`, `select`, `button`, `a`) and style them, so keyboard and screen-reader behaviour stays native.
- **Do** keep text at 4.5:1 and non-text state at 3:1 in both themes. Check the dark theme separately.
- **Do** use logical properties and test RTL. Shortcuts keep LTR key order.

### Don't:
- **Don't** make it look like lichess (no lichess colours, logo or layout). The app is unofficial.
- **Don't** use the accent for decoration, or add a second accent colour.
- **Don't** add shadows to list boxes, or nest list boxes in cards.
- **Don't** add a display font, eyebrow labels above headings, or emoji/Unicode glyphs as icons. Icons are authored SVG at 16px.
- **Don't** add motion beyond 160ms state transitions. Everything is disabled under `prefers-reduced-motion`.
