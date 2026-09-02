# Bottom Pane UI Review

**Mode:** Explore

## Intent

The bottom pane works but isn't polished. Review its UI broadly — easier to use, nicer to look at. On the table:

- Rework the colour scheme of each pane.
- Graceful degradation to narrower screens.
- Move the playlist's deck indicators from the left-hand side, where they eat several columns, to the right, partially overlaying the track name — the name is visible on the deck it's loaded to anyway.
- An extra row on top of the browser (below the bottom deck) for cross-pane navigation.
- Pane borders that all line up.
- Any possible simplification of the keys.

(2026-09-01, reworked from bottom-pane-tabs.)

## Approach

### The nav row replaces the slivers

A one-row strip over the browser pane, just below the lowest deck, names the three panes: the active one highlighted, inactive ones carrying their nav hint (`H`/`L`). It carries orientation and — on narrow terminals — the indication of the off-screen pane, so the sliver mechanism (its columns, its layout branch, its vertical text) is deleted.

### One shared pane frame

All three panes render through a single frame renderer with title-in-border anatomy — cheapest vertically, and border alignment then holds by construction rather than per-pane tuning.

### Activation lives in the chrome

The active pane is marked by its accent-bright border and title; inactive panes render normally but quiet. The per-pane content dim goes — the wide layout exists to read all three panes at once. The whole-region dim for an unfocused Bottom Pane stays: that is the view-focus signal, not pane activation. Bottom-pane activation visuals are owned here; focus-highlighting keeps the global deck-vs-pane question.

### Per-pane hue identity

Each pane gets a family colour used in its border, title, and highlights. The browser's colour is already mode-driven (command amber, search cyan, move blue) and stays so — its identity is "the pane whose hue signals mode". Playlist and tags get fixed families that don't collide with the browser's three mode hues: green for the playlist, violet for the tags. Activation brightens the family rather than changing it.

### Deck badges right-aligned

The playlist's deck indicators become right-aligned badges over the end of the track name; rows without a badge keep the full name width. The overlaid characters stay readable on the loaded deck itself.

### Degradation is a ladder

Three panes → two → browser-only, driven by per-pane minimum useful column floors instead of the single 120-column threshold — percentages alone let panes go uselessly thin before a cliff trips. The floor values are settled by feel during Build.

### Keys regularised afterwards

Once the nav row and chrome land: `H`/`L` unchanged (lowercase belongs to browser navigation), `Esc` always steps toward the browser, pane-local keys stay pane-local, and anomalies like the picker's `h`-to-close are folded in. No new chords.

## Plan

**Topics**

- Shared pane frame; borders align across all three panes

- Nav row over the browser (names, active highlight, `H`/`L` hints); slivers deleted

- Activation and hue in the chrome: per-pane families (playlist green, tags violet, browser mode-driven), no per-pane content dim

- Playlist deck badges right-aligned over the name end

- Degradation ladder: three → two → browser-only on column floors, values by feel

- Key regularisation: `Esc` steps toward the browser, picker's `h`-to-close folded in

- Live evaluation and iteration by feel

**Done when** the bottom pane reads as one coherent three-pane surface — borders aligned, nav row carrying orientation, activation and identity in the chrome, badges moved, narrowing degrading through the ladder — or an element is rejected with reasons recorded.

## Log

- First chrome build (0.37.1): shared header/frame/footer anatomy in `render::pane_*` helpers; browser and playlist already had the three rows, tags gained them. Nav row replaces the browser's top-bar mode content; search/move/name-prompt inputs relocated to the browser footer beside the mode chip. Slivers and per-pane content dims deleted; the whole-region unfocused dim stays.
- The browser top bar's workspace hint ("workspace set · ' clear") was dropped rather than relocated — the border title already shows the workspace and the command legend lists `@`.
- Deck badges: `◂N` now right-aligned over the name tail (badge bg = playlist green), only badged rows lose name columns; the left marker column and its 2-column cost are gone.
- The candidate picker (`render_confirm`) still wears its old chrome — not yet on the shared anatomy.
- Feedback round (0.37.2): rename banner truncates the filename, keeping `[y]` and the countdown visible; `y` now accepts the offer with the browser focused (command mode only — search types it, move mode's `y` confirms the move), via a shared `accept_rename_offer!`. Deck chip removed from the nav row (the highlighted deck carries the information); nav items left-justified as `Name [H/L]`. Deck badge arrow flipped to `▸N`. Workspace hint restored as a command-footer suffix ("ws set · ' clears") — provisional home. Arrows mirrored where clash-free: playlist Shift+↑/↓ reorder, →/← step back to the browser from playlist/tags; ←/→ can't mean pane movement in the browser (directory navigation owns them).
- Rename offer redesigned (0.37.3): the prompt lives only on the deck corner (countdown, then lingering ⚠) — the bottom-pane banner and the 0.37.2 browser-side `y` are gone; `y` accepts from deck focus only. Accepting summons the full fix flow: browser view up and focused at the file with compliance markers on, tag editor open, Tags pane active; the editor closing hands activation back to the browser. `L`/`e` editor opens now also mark the Tags pane active.
- Ladder settled by feel: the three→two boundary stays at 120 columns; under 50 the layout is a carousel — the active pane alone at full width, `H`/`L` walking between all three (0.37.5, reworked from 0.37.4's browser-only single mode on feedback). a/A inserts stay off while the playlist is off-screen.
- "Arriving is editing" rejected for the tags side (0.37.6): the modal editor swallowed `H`, breaking the walk symmetry the moment you arrived. `L` now only activates the Tags pane; `e` opens the editor from it — the same key as the browser's edit, making `e` the one editing key everywhere. `H`/`L` are a symmetric walk in every layout.
- Nav strip distributed (0.37.7): the header state texts ("Playlist · pinned", "Preview · follows the highlight", the edit banner) are gone — each pane's name-and-key tip now sits above its own top-left corner, and off-screen panes' tips dock at the nearest screen edge, compressing instead of vanishing as the layout narrows. The browser no longer owns the nav row; the jumped-to location label moved to the footer's right end. Pinned-ness now reads from the tip's colour (green vs grey).
