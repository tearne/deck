# Record-Box Playlists

**Mode:** Formal

*(Part of the loop-rethink sequence — see the design for ordering and context.)*

## Intent

A playlist is the box of records planned for the set, and nothing more: a picker, sending any entry to any deck. The deck-side attachment goes wholesale — no auto-advance at end of track, no next/previous stepping, no playlist state on a deck (badge, entry tracking, attachment restore). Resolution stays: the box must still tell the operator which records can't be found. A future opt-in auto-play mode may bring sequential play back; it is explicitly not built here. The `playlist_next`/`playlist_prev` actions and their plumbing are deleted.

The box must be quick to reach and normally stay open — mechanism to be settled (perhaps an alternative view to the browser or the tag panel). Entries loaded on a deck are marked in the box (arrows or similar naming the deck).

Design: [loop-rethink](../archive/2026-08-15-loop-rethink.md). (2026-08-29 rewrite supersedes the 2026-08-22 two-modes note: only the picker exists for now.)

## Approach

### The box lives in the context panel, pinned

The browser's panel gains a second sticky state: besides passively following the highlight (tags preview), it can hold the **current box** pinned. Tag-gazing and box-work are mutually exclusive activities, so they share the space. The browser view already persists in the bottom pane, so the box is one `open_browser` chord away at all times — "quick to reach, normally open" without a new view.

### One current box, pinned and persistent

Opening a playlist in the browser pins it as the current box; a key toggles the panel between follow and box without unpinning. Pin state and the current box persist in Session State. No box pinned means the panel simply follows, as today.

### Picking is panel focus

The existing panel-focus route drives the box: focus it, `j`/`k`, `Enter` sends the entry to the selected deck. Deck marks (`◂n`, computed per frame by path against the decks) and unplayable flags render live even while the panel is unfocused.

### Adding, editing and fixing stay in the panel

The transactional editor opens on the pinned box with `e` and commits back to it; inserting from the browser is the existing flow. `Enter` on an unresolved entry opens the candidate picker in place.

### The deck-side attachment is deleted wholesale

`ActivePlaylist`, the deck's playlist field, auto-advance, `play_playlist_step`, the `playlist_next`/`playlist_prev` actions, the badge, deck-side unplayable warnings, and the snapshot's playlist fields all go. Old session files still load (dropped fields ignored). `Enter` on a playlist in the browser now pins it as the box — nothing loads onto a deck.

### Resolution happens at the box

Pinning a box resolves every entry with the existing whole-set resolution; unresolved entries render flagged and can't be sent.

## Plan

- [x] Current box + panel pin state, persisted in Session State and restored on start
- [x] Pinned box rendering: entries with deck marks (`◂n` by path) and unplayable flags, live while unfocused
- [x] `b` in browser command mode toggles the panel follow ↔ box
- [x] `Enter` on a playlist in the browser pins it as the current box (no deck load)
- [x] Panel-focused picking on the pinned box: `Enter` sends to the selected deck, keyboard follows the load
- [x] `e` on the pinned box opens the transactional editor; commit and cancel land back on the pinned box
- [x] `Enter` on an unresolved entry opens the candidate picker in place
- [x] Whole-set resolution on pin
- [x] Delete the deck-side attachment: `ActivePlaylist`, the deck playlist field, auto-advance, `play_playlist_step`, `restored_playlist`, `open_playlist_on_deck`, the badge, deck-side unplayable warnings, `playlist_next`/`playlist_prev` actions, snapshot playlist fields (old sessions still load)
- [x] keybindings.md and config comments caught up (`b`, deleted actions)

## Log

- `Enter` on a highlighted playlist was consumed by the panel machine (transient browse) before the browser could see it — the transient-browse binding is now `l` only, freeing `Enter` to pin.
- `l` was too overloaded to also focus the box (it already enters directories and browses highlighted playlists); `b` therefore both shows-and-focuses the box, and from inside puts it away. Esc steps out one level, box still showing.
- Deck marks match by content identity (the deck's analysis hash against the entry's), so renames don't unmark; a just-loaded track is unmarked for the moment before its hash lands.
- Deleted alongside the plan's list: the attachment-era resolution helpers (`resolve_playlist`, `resolve_and_heal`, `PlaylistResolution`), the panel's `unplayable`/`next_up`, and the workspace-set deck-healing loop (panel and box still re-resolve). One badge test deleted, one adapted to the box's unresolved count.
- The workspace-set "Relocated moved tracks" success message went with the deck-healing loop — healing still happens in the panel/box recompute, silently.
- Hand-back: box `Enter` could silently overwrite a playing deck — panel loads now route through the browser's shared confirm gate (`play_panel_entry` became `resolve_panel_entry`; one loader).
- Hand-back: transient `l`-browse removed — hover previews, `Enter` pins, `e` edits; the panel gained a one-row header naming its state with a key tip ("Preview · Enter pins", "Box · b enters", …), aligning its border with the browser's.
- Hand-back: headers say "Playlist" not "Box"; hint strip one row so the outline's bottom aligns with the browser; `l` on a playlist pins like Enter (vim "into"); `record_box` (`space+b`) reaches the box from the decks (summoning the browser) and toggles like `b` inside it. Neither b nor space+b works in search mode — Space types into the filter there, per the bottom-pane decision.
- Known trade-off, surfaced late: while the pinned playlist is shown, the hover tags preview is suppressed (the panel is occupied). Put the playlist away (`b`/`space+b`) to get tags back.

## Conclusion

Completed. Shipped as 0.35.3 (minor bump confirmed).
