# Panel Layout Review

**Mode:** Explore

## Intent

The browser/panel layout is logical but not naturally ergonomic; experiment with alternatives.

Leading idea — **three sliding panes**: Tags, Browser, Playlist side by side in the bottom pane, each with its own border, header, and a footer for navigation and usage tips. Two panes show at a time (Tags+Browser, or Browser+Playlist); the off-screen pane leaves a sliver at the edge indicating its existence. `h`/`l` slide the panes based on activation — likely lowercase for in-pane movement and `H`/`L` for sliding/activation. (2026-08-30)

## Approach

### Three peer panes replace browser-plus-appendage

Tags, Browser, Playlist become equal panes in the bottom pane's browser view, each with border, header (name and state), and footer (keys) — the record-box header generalised. The context panel stops being a browser internal; notably, Tags and Playlist stop competing for one slot, dissolving the suppressed-tags trade-off from record-box-playlists.

### A visible pair and a sliver

Two panes show at once — Tags+Browser or Browser+Playlist — the off-screen pane reduced to a sliver at the edge naming itself vertically. The existing panel width percentage governs the pair's split; the sliver is a fixed few columns.

### Case pairs the keys: h/l move, H/L slide

Lowercase stays in-pane (cursor, directories). `H`/`L` move activation left/right across the three panes; the visible pair follows the activated pane. Tab and Esc keep their global meanings.

### Existing states get pane homes

Hover tags render in the Tags pane (always, no suppression); the pinned playlist is the Playlist pane's content; the transactional editor and candidate picker live in the Playlist pane; the tag editor and rename-offer flow live in the Tags pane. `b`/`space+b` become "activate the Playlist pane".

## Topics

- Chrome: borders, headers, footers, sliver rendering for all three panes
- Slide mechanics and activation model (H/L, where b/space+b land)
- State migration: hover tags, pin, editor, picker, rename offer
- Live evaluation and iteration by feel

**Done when** the sliding three-pane layout feels ergonomic in use with every existing flow rehomed — or the experiment is rejected with reasons recorded.

### The Browser+Playlist arrangement is the editing mode

Slid right, the pair itself is the playlist-working mode: no separate edit state. Editing keys are live in that arrangement, the browser beside the playlist as the source of records.

### Edits write live

No transactional buffer: inserts, reorders and removals write to the `.rpl` as they happen, through the existing atomic write. The arrangement is the mode; there is nothing to commit or abort.

## Conclusion

Accepted: the three-pane layout replaces browser-plus-appendage. Beyond the planned scope, wide terminals (≥120 columns) show all three panes at once, sides swapped to Playlist | Browser | Tags, and `b`/`space+b` were retired outright rather than rehomed. Ships as 0.36.7 (minor bump confirmed). The unified-chrome topic passes to the open UI changes (bottom-pane-tabs, focus-highlighting). Map catch-up needed: Bottom Pane, Browser, Context Panel, Playlist Editing, and Candidate Picker describe the old model; keybindings.md is already current.

## Log

- First build: three panes with pair-plus-sliver geometry (`H`/`L` activation, browser always visible, companion share = the old panel width). The old Panel state machine (Preview/Browse/Edit/Confirm) is deleted; the picker survives as an overlay state on the Playlist pane; `box_shown` died — the Playlist pane always exists, `b`/`space+b` toggle activation and slide.
- Live edits: insert/reorder/remove write through `commit_playlist` immediately; the transactional editor and `EditFocus` are gone.
- Hovering a `.rpl` previews it in the companion pane of the left pair ("Preview · Enter pins"); `l` on a playlist pins like Enter.
- The Tags pane is currently chrome-light (no border/header yet — the browser and playlist panes carry their own); unified chrome is the next feel iteration.
- The tags pair's companion width is a fixed 30% (the playlist pair keeps the adjustable panel width); `H` from the browser now opens the highlighted track's tag editor directly — arriving is editing, mirroring the playlist pair. Non-track highlight falls back to activating the inert Tags pane.
- The tag editor's hint now reads "Enter saves and exits · Esc exits without saving" (brightened) — with `H` opening the editor, `L` types a capital L rather than sliding, so the exit route needs to be explicit.
- `H` from the browser while slid right now only slides the tags pair back into view; the editor opens on a further `H` once the tags pair is already showing.
- Both pair widths are adjustable and persisted separately: alt+h/l steps the companion of whichever pair is in view; both default to 25% (a stored playlist width from an earlier session carries over until re-adjusted).
- Sides swapped: the order is now Playlist | Browser | Tags, and `H`/`L` directions flip with it — `H` toward the playlist, `L` toward the tags, with `L` from the browser (tags pane showing) opening the tag editor.
- On terminals ≥120 columns all three panes render at once (no slivers, no sliding); below that the pair-plus-sliver behaviour remains. With three across, a/A insert works whenever the browser is active. An open tag editor forces the tags pane on screen in narrow mode (previously it could be open but off-screen).
- Alt+h/l reworked as a boundary drag: they move the active pane's right-hand boundary left/right — playlist/browser divider from the playlist, browser/tags divider from the browser; nothing from Tags. The tag editor now ignores alt/ctrl character chords instead of typing them into the field.
- `b` and `space+b` (the record_box action) are removed — `H`/`L` cover pane access. The deck-side "summon browser straight to the playlist" shortcut goes with it (space+f then H now). A stale config's `record_box` line is skipped with a startup warning.
