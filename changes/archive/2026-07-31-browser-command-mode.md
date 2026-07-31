# Browser Command Mode

**Mode:** Formal

## Intent

The browser is out of keys. With a workspace set, every letter is fuzzy-search input, so no letters are free for commands — navigation and actions have to be shoehorned in (`j`/`k` work only when there's no workspace; the move-destination picker is a bespoke sub-mode). Adding the playlist commands (open, edit, create) would make this worse.

Make the browser modal, in the Helix style already liked elsewhere: `Tab` toggles between **search mode** (letters filter the list) and **command mode** (letters are commands — navigate with `j`/`k`, and act). Command mode gives room for the growing command set and a clear home for playlist keys, and the existing workarounds fold back into the model.

Make the mode unmistakable: a status/hint bar showing the current mode and its available keys, and a distinct colour theme per mode (search, command, playlist, move) — generalising the blue treatment the move picker already uses. Modal UIs live or die on signalling which mode you're in, so this is core, not polish.

Foundational, built before the playlist browser-integration so those commands drop into command mode cleanly rather than piling on more special cases. Whether the player's file-operations submenu (tag edit, move) also belongs in browser command mode is a follow-on question this sets up, not settled here.


## Approach

### A mode enum drives input and rendering

Replace the ad-hoc flags (`pick_destination`, the implicit "search is active when the term is non-empty") with a single `BrowserMode` — `Command`, `Search`, `Move` — extensible for `Playlist` when playlists arrive. Both key dispatch and rendering switch on it: each mode owns its key table and its look, so the scattered guards disappear.

### Tab toggles Command and Search; actions enter sub-modes

`Tab` switches Command↔Search. Command-mode action keys enter the focused sub-modes (`Move` now, `Playlist` later). `Esc` always exits the browser to the player, from any mode — one consistent rule (and it restores one-press move-cancel). Switching modes without leaving is `Tab`. The browser opens in Command mode at session start; thereafter it reopens in whichever primary mode (Command or Search) was active when it last closed — the last mode is remembered in the player loop across browser open/close, so `space+f` restores where you left off. Sub-modes (Move, Playlist) are transient: they always collapse to their primary on close, so a reopen never lands in one. `Tab` (or `/`) enters Search.

### Search filters the workspace, or the current directory

Search mode filters the workspace recursively when one is set, else the current directory's listing — generalising today's workspace-only search, matching the intent that search works either way. Letters only ever filter in Search mode; in Command mode they are commands.

### j/k unconditional; move becomes a mode

With letters no longer stealing keys in Command mode, `j`/`k` (and arrows) navigate always — dropping the "only without a workspace" guard. The move-destination picker stops being a bolt-on flag and becomes the `Move` mode. Both existing workarounds dissolve into the model.

### Per-mode colour theme

Each mode carries a distinct accent — border, cursor highlight, and status bar — generalising the blue the move picker already uses. Distinct hues so the mode reads at a glance (Command amber, Search cyan, Move blue; Playlist later). Exact palette settled during build.

### Persistent mode status/hint bar

The bottom line always shows the active mode's name and its key legend, replacing the single ad-hoc hint. This is the primary discoverability mechanism — a modal UI must announce which mode it is in and what the keys do.


## Plan

- [x] Replace `pick_destination` with a `BrowserMode` (Command / Search / Move) on `BrowserState`.
- [x] Dispatch key handling by mode, moving the existing per-guard logic into per-mode key tables.
- [x] `Tab` toggles Command↔Search; `/` enters Search; the move action enters Move; `Esc` steps sub-mode→Command and Command→player.
- [x] Search mode filters the workspace recursively, or the current directory when no workspace is set.
- [x] Remember the last primary mode in the player loop and restore it on reopen (Command at session start; sub-modes exit on close).
- [x] Per-mode colour theme for border, cursor highlight, and status bar.
- [x] Persistent status bar showing the active mode and its key legend.
- [x] Bump Cargo patch (0.11.22 → 0.11.23).


## Log

- `handle_browser_key` now dispatches on `state.mode` into `command_key` / `search_key` / `move_key`, each a small key table. The old scattered guards (`workspace.is_some()`, `search_term.is_empty()`, `pick_destination`) are gone; navigation/open/up factored into shared helpers.
- Command mode keys: `j`/`k`/arrows nav, `Enter`/`l` open, `h`/`Left`/`Backspace` up, `/` or `Tab` → Search, `@`/`'` workspace, `q` quit app, `Esc` → player. Search: letters filter, arrows nav, `Enter` open, `Tab` → Command, `Esc` clears + → Command. Move: dir-only nav, `y` confirm, `Esc` → Command.
- Search without a workspace now fuzzy-filters the current directory's entries (new `update_search` branch); with a workspace it's the existing recursive search.
- Mode restore: `last_browser_mode` in the player loop, set to `bs.primary_mode()` on every browser key, applied to a fresh `BrowserState` on open. Move entry (file-ops `m`) sets `mode = Move` directly, overriding the remembered mode. `#` preview gated to Command mode so `#` types in Search.
- Theming: `mode_theme` gives each mode an accent (Command amber, Search cyan, Move blue) applied to border, cursor highlight, and a new status bar showing `[MODE]` + the mode's key legend (replaces the old single hint line). Launch smoke-tested (no panic); interactive behaviour is for hand-back.
- Hand-back revision (0.11.24): dropped `q`-quits-the-app from the browser (surprising/dangerous) — the `BrowserResult::Quit` variant and its main handler are removed. `Esc` now exits to the player from every mode instead of stepping sub-mode→Command; simpler consistent rule, and it restores one-press move-cancel. Legends updated; switch modes without leaving via `Tab`.


## Conclusion

Shipped at 0.11.24. The browser is modal: `BrowserMode` (Command / Search / Move) drives both input and rendering, with `Tab` toggling the primary modes, `Esc` exiting from any mode, and per-mode colour + a status-bar legend for signalling. Two prior workarounds dissolved — `j`/`k` navigate unconditionally, and the move picker is a real mode rather than a bolt-on flag. Search also gained current-directory filtering when no workspace is set, and the primary mode is sticky across reopen. `q`-quits-app was removed during hand-back.

This is the foundation it set out to be: playlist commands (open, edit, create) now have a home as command-mode keys, and the `Unresolved` browser-side questions in [[playlist-editor]] get simpler for it. Follow-on left open (per the Intent): whether the player's `~` file-operations submenu belongs in browser command mode too.

Map catch-up pending: the Browser node (and likely Search) should describe the modal model — command/search/move, `Tab`/`Esc`, the sticky primary mode, and workspace-less search — replacing the current single-mode description.

