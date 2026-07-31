# Browser File Operations

**Mode:** Formal

## Intent

Move file operations out of the player's `~` which-key submenu and into the browser's command mode, where a file manager's operations belong. `e` (edit tags / rename) and `m` (move) become command-mode keys acting on the **highlighted entry** — any file, not just the track loaded on a deck. The `~` submenu is then removed.

This is a more general model (edit or move any file you can navigate to) and finishes a job already half-done — move already launches the browser today. The trade: to act on the currently-playing track you navigate to it in the browser (easy — you loaded it from there, and the browser reopens where you left off) rather than a player shortcut.

Two pieces of work sit underneath it:

- **Decouple the tag editor from decks.** It currently lives on `Deck` (`d.tag_editor`, handled by iterating decks); editing an arbitrary browser file needs it as a standalone overlay, like the browser itself. A good cleanup regardless.
- **Deck-sync on touch.** Renaming or moving a file that happens to be loaded on a deck must update that deck's path and rename hint — the sync the move already performs. The load-time rename offer opens the decoupled editor.

Foundation for [[browser-tag-compliance]]: fixing flagged files in place needs in-browser editing.


## Approach

### Tag editor becomes a standalone overlay

Move the editor state off `Deck` (`d.tag_editor`) to a single `Option<TagEditorState>` in the player loop, mirroring `browser_state`: rendered and key-handled at the top level, capturing input while open. The state carries the full file path (parent + stem + extension) so save is self-contained. It can be open over the browser or over the player, and closing returns to whatever is beneath — so browser editing flows back to the listing for the next file (the compliance loop).

### Save writes and renames the file, independent of any deck

The save path (`write_tags`, then `fs::rename` when the stem changed) operates on the editor's own path, not a deck's. Outcome notifications move from the deck (`d.active_notification`) to the global notification.

### Deck-sync when a touched file is loaded

A rename or move produces a new path; a shared helper finds the deck (if any) whose `path` equals the old one and updates its `path`, `filename`, `track_name`, and rename hint. So editing or moving the track you're currently hearing keeps its deck correct — the sync `move_track_to_directory` already does, generalised and shared. Audio is decoded in memory, so playback is unaffected.

### Browser command-mode `e` and `m` act on the highlighted entry

In command mode, `e` opens the standalone editor for the highlighted audio file; `m` enters Move mode to relocate it. Both no-op on directories and non-audio entries. After an edit-rename or move, the browser re-reads its listing so the change shows.

### Move mode carries its source file

Move mode currently relocates a deck's loaded track; instead it carries the file highlighted when `m` was pressed, and `DirectoryChosen` moves that file. Move is scoped to audio files (directory moves are out of scope).

### The `~` submenu is removed; the rename offer opens the standalone editor

The player's `FileOperations` action, the `~` binding, `file_ops_menu_open`, and `render_file_ops_menu` all go. The load-time rename offer opens the top-level editor for the deck's track path instead of the deck-bound one.


## Plan

- [x] Move the tag editor to a top-level `Option<TagEditorState>` (carrying the file's directory), off `Deck`.
- [x] Render and key-handle the editor at the top level; save on its own path via `write_tags` + rename, reporting through the global notification.
- [x] Deck-sync helper updating any deck loaded from a renamed or moved file (path, filename, track_name, rename hint); used by save and by move.
- [x] Browser command mode: `e` opens the editor for the highlighted audio entry, `m` enters Move for it; no-op on directories and non-audio.
- [x] Move mode relocates its carried source file; refresh the browser listing after an edit-rename or move.
- [x] Load-time rename offer opens the top-level editor.
- [x] Remove the `~` submenu: `FileOperations` action and binding, `file_ops_menu_open`, `render_file_ops_menu`.
- [x] Bump Cargo patch (0.11.24 → 0.11.25).


## Log

- Tag editor decoupled: `TagEditorState` now carries the file's `dir` (plus `current_path`/`target_path` helpers) and lives as a top-level `tag_editor: Option<_>` in the loop, rendered and key-handled at the top level via `handle_tag_editor_key`. Off `Deck` entirely.
- Save is deck-independent: writes tags + renames the editor's own file; `sync_deck_path` then updates any deck loaded from the old path (path/filename/track_name/rename hint); notifications went from per-deck to global. If the browser is open under the editor, it refreshes on rename.
- Move generalised: `move_file_to_directory(source, dest)` (was deck-based `move_track_to_directory`) returns the new path; browser `m` carries the highlighted entry as `move_source`, Move mode navigates carrying it, `DirectoryChosen` moves it + deck-syncs + refreshes and stays in the browser (back to Command).
- Esc-from-Move now returns to Command (move is browser-initiated now), not straight to the player — a small refinement of the "Esc exits" rule for the new flow. Flag on hand-back.
- `~` fully removed (action, binding, submenu render + handling); help footer and keybindings.md updated. Map catch-up pending: the File Operations node subtree (submenu → browser command-mode ops), Move node, and the Keymap fixed-keys line.


## Conclusion

Shipped at 0.11.25. File operations moved from the player's `~` which-key submenu into the browser's command mode: `e` edits/renames and `m` moves the highlighted audio entry (any file, not just a loaded track). The tag editor is now a standalone top-level overlay decoupled from `Deck`; save writes/renames the file itself, and a shared `sync_deck_path` updates any deck loaded from a touched file (the same follow-the-file behaviour move had). The `~` submenu, its action, and binding are gone.

Two small refinements from hand-back: Esc from Move returns to Command (move is browser-initiated now) rather than exiting the browser; the `~` key is freed. A follow-on was captured: [[deck-independent-browser]] — the browser is still opened *for* a deck though its file ops aren't deck-specific, so loading should pick the deck instead.

Map catch-up pending (enumerated separately): the File Operations subtree no longer describes reality — the submenu is gone and edit/move are browser command-mode operations.
