# File Operations

**Mode:** Formal

## Intent

Track files can only be renamed or retagged when a non-conforming filename happens to prompt at load; there is no on-demand way in, and no way to move files at all. Add a file-operations submenu — a submenu because the keymap is running out of keys — offering:

- **Rename** and **tag editing** (the existing editor), callable on demand for any track, not just when prompted at first load.
- **Move to directory** — relocate the current track between directories, e.g. sorting tracks while listening.

The submenu should take cues from Helix's approach: a prefix key opens a transient panel listing the available keys with their action names; the next keypress executes, Esc cancels.

Open question for the Approach: when a move breaks a known playlist's paths, should playlists be updated eagerly, or left to their own resilient resolution at next load?

This supersedes the `tag-editor-invocation` placeholder (on-demand editor invocation, configurable entry key) — its scope is contained here.


## Approach

### Which-key submenu, prefix configurable, inner keys fixed

A configurable prefix action (`file_operations`, default `~`) opens a transient overlay listing each operation's key and name, Helix-style; the next keypress executes or Esc/prefix-again cancels. Inner keys are fixed rather than keymap entries — the submenu exists to stop consuming keymap space, and fixed keys can't collide with it. `~` frees up by moving `palette_cycle` to `p`.

### One editor entry covers rename and tags

The submenu's edit entry opens the existing tag editor (which already renames from its fields) for the selected deck's track — the same state the rename offer's `y` constructs. Rename and tag editing are one operation in this app; the submenu doesn't split them.

### Move destination picked with the browser

The move entry opens the existing browser in a pick-destination mode: navigation as normal, `y` (shown in the browser footer) selects the currently open directory as the destination, Esc cancels. `y` confirms the open directory rather than the highlighted one because Enter is already how the browser descends into a subdirectory — confirming the directory you're standing in leaves navigation intact. Reusing the browser avoids a second directory-navigation UI.

### Move is a rename; the deck barely notices

Moving executes `fs::rename` (same filesystem — a cross-device move fails with a clear notification rather than a copy fallback, which would need progress and interruption handling disproportionate to the sorting use-case). Audio is fully decoded in memory, so playback is unaffected; the deck's path and rename hint update, and the cache follows the content hash, so no cache surgery.

### Playlists are left to their own resolution

A move updates no playlists. The playlist spec's file-resolution re-links moved files automatically (library-root search, content-hash confirm) — moving within the library is its designed case — and Deck has no playlist implementation yet in any case (that's the open `playlist-editor` change). Eager rewriting would duplicate spec machinery for no benefit.


## Plan

- [x] Move `palette_cycle` to `p`; update the help overlay and keybindings.md.
- [x] Add a `file_operations` action bound to `~` that opens the submenu.
- [x] Render the transient which-key overlay — one row per operation (key + name).
- [x] Submenu input: edit key opens the tag editor, move key opens the browser picker, Esc or `~` cancels.
- [x] Extract tag-editor-state construction so it opens on demand for the selected deck (reused by the rename offer).
- [x] Browser pick-destination mode: `y` confirms the open directory, footer shows the hint, Esc cancels.
- [x] Execute the move with `fs::rename`; cross-device failure raises a notification; update the deck path and rename hint.
- [x] Bump Cargo patch (0.11.10 → 0.11.11).


## Log

- Rename hint doesn't need touching on move: it derives from the filename stem, which a directory move preserves. Plan task wording said "update the deck path and rename hint"; only the path changes.
- Move guards added beyond the plan: same-directory move is a warning no-op, and an existing file at the destination is refused rather than overwritten.
- `q` is suppressed in the browser's pick-destination mode so an accidental press can't quit the app mid-move; Esc is the cancel.
- Dismissing the submenu with Esc armed the quit-while-playing warning (0.11.12 fix): a paired Esc Press (crossterm + Kitty decode) landed on Action::Quit after the menu closed. Now sets `suppress_quit_until` on Esc dismiss, matching the help overlay. Root-cause refactor spun off as `input-event-normalisation`.
- 0.11.13: pick-destination browser gets a distinct blue scheme — folders (choosable) in clear blue, tracks/other files dimmed blue to signal non-selectable, blue cursor highlight, blue banner, and a move-specific footer hint.
- 0.11.14: in pick mode the cursor now stops only on folders (was resting on tracks and giving them the bright highlight, reading brighter than the dirs). Added j/k as ↓/↑ aliases, active where letters aren't search input (no workspace) and always while picking.


## Conclusion

Shipped at 0.11.14 (patch cadence held across the hand-back iterations). Beyond the plan: guards on the move (same-dir no-op, refuse overwrite, cross-device notification), the Esc-dismiss quit-warning fix, a blue pick-destination scheme with folder-only cursor stops, and j/k navigation. Two follow-ons recorded: `input-event-normalisation` (root-cause refactor for the recurring paired-Esc quirk) and the superseded `tag-editor-invocation` placeholder (deleted). Also surfaced `Y/^ fps` in the help overlay, previously undocumented there. Map catch-up pending — nodes enumerated separately.
