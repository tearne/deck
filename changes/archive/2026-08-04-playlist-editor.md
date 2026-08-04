# Playlist Editor

**Mode:** Formal

## Intent

The operator experience of playlist support, built on the playlist engine ([[playlist-format]]): opening playlists from the browser, per-deck playlist state, a permanent context panel (track metadata and playlist contents, with browse-to-play and transactional editing), creating playlists, a position indicator, and auto-advance between tracks. File resolution uses Deck's `@` workspace as the library root, and the descriptive-fallback confirmation is presented to the operator.

UI and wiring only — the `.rpl` engine (format, resolution, migration, resilient writes) is [[playlist-format]], which in turn uses the `resilient-playlists` crate for identity. This change supplies the real library-lister and tag-reader the engine's resolution needs, and presents its outcomes.


## Approach

### Browser treats `.rpl` files as selectable

Add a `BrowserEntry::Playlist` variant. `.rpl` files are highlighted in the browser listing (parallel to audio files) and selectable. Selecting one opens a playlist on the target deck and loads the first resolvable track. Non-resolvable entries are skipped to the next.

### Deck carries optional playlist state

Add `playlist: Option<ActivePlaylist>` to the `Deck` struct, where `ActivePlaylist` holds the loaded `Playlist` and the current index. All three decks can independently hold playlists. When a deck is cleared or a standalone track is loaded, its playlist state is dropped.

### Permanent context panel

The browser is permanently split ~70/30: the file browser on the left, a context panel on the right. The panel reflects the highlighted entry — a track's metadata, or a playlist's contents (entries with description, resolution status, and the ▶ playing / ⇢ next-up markers) — and is empty on a directory or other file. This replaces the hover-toggled editor and the separate full-screen tag modal.

### Three panel states, distinct colours

- **Preview** — the browser is focused; the panel displays the highlighted item read-only. Neutral colour.
- **Browse** — `l` / `Enter` focuses the panel read-only (playlists only): `j/k` navigate the entries and `Enter` loads the highlighted entry onto the target deck, attaching that playlist at that index — so any position of any playlist can be jumped to directly, regardless of what's currently on the deck. A subtle focus colour.
- **Edit** — `e` focuses the panel for change, transactional, with a prominent `Enter → Commit · Esc → Abort` banner and a distinct edit colour.

### Transactional playlist edit

Edit works on a buffered copy of the playlist. Insert-at-point places a track — or another playlist's entries, spliced in — from the browser at the panel's cursor; reorder and remove act on the buffer. `Enter` commits: the `.rpl` is written and, if it's loaded on a deck, the change is adopted there (playing track kept by identity). `Esc` discards the whole session. This replaces append-only edits and per-edit live writes.

### Tag editing in the panel

`e` on a track edits its tags in the panel rather than a full-screen modal, reflowing to the narrow width. `Enter` saves, `Esc` cancels. The content-identity safeguard on write is unchanged.

### Visual indicator in the deck info bar

When a deck has an active playlist, the current position `x / y` is shown in the info bar alongside the track name. A short prefix label distinguishes playlist mode from standalone loading.

### Auto-advance hooks into `service_deck_frame`

When a deck's track finishes (remaining time ≤ 0, not paused) and its active playlist has a next entry, a load is triggered automatically. Each entry is resolved lazily — just before it plays, via the engine — and the file is rewritten on a successful relocate.

### File resolution uses `@` workspace as library root, confirmation in-app

The existing workspace (`BrowserState.workspace`, persisted in cache) becomes the engine's library root — Deck's library-lister enumerates it. If no workspace is set, resolution falls back to hint-only (no library search); the existing workspace prompt (`@`) serves this purpose. When the engine returns a needs-confirmation outcome (descriptive fallback), Deck presents the ranked candidates for the operator to confirm or reject.

### No-workspace nudge and auto-heal on workspace set

When an open playlist has unavailable entries and no workspace is set, surface a nudge that setting a workspace (`@`) will relocate moved tracks — so the operator understands why entries are dark and how to fix it. Setting or changing the workspace then re-resolves the open playlists' currently-unavailable entries (found entries need nothing), updates their displayed status, and persists rewritten hints for any that relocate — the payoff is immediate. Re-resolution scans and hashes candidate files, so for a large library it should heal only the unavailable entries and avoid blocking the UI (a brief "relocating…" indication, or off-thread). The engine already supports this: re-resolving is calling `resolve` again with a now-populated library, taking the returned updated entry to persist.


## Plan

Playback (built):

- [x] Implement Deck's `Library` (enumerates the `@` workspace; hint-only when unset) and a resolve helper that supplies track facts
- [x] Add `ActivePlaylist` (loaded `Playlist` + current index) to `Deck`; drop it on clear or standalone load
- [x] Browser: add a `Playlist` entry variant, highlight `.rpl` files, and on select open the playlist on the target deck, loading the first resolvable entry (skipping unresolvable)
- [x] Deck info bar shows `x / y` position with a playlist-mode prefix when a playlist is active
- [x] Auto-advance in `service_deck_frame`: on track end with a next entry, lazily resolve and load it, rewriting the file on relocate

Playback extras (built):

- [x] Manual skip on the selected deck: `playlist_next` / `playlist_prev` to the next/previous resolvable entry
- [x] No-workspace nudge when entries are unavailable; on workspace set, re-resolve and persist relocated hints

Context panel — permanent split (replaces the built hover editor, which is torn out):

- [x] Permanent ~70/30 browser split: file browser (left) + context panel (right), always shown
- [x] Preview: highlighted track → its metadata; highlighted playlist → its contents (with ▶/⇢ markers)
- [x] Browse state: `l`/Enter focuses a playlist's entries read-only; `j/k` navigate; `Enter` loads the highlighted entry onto the target deck (playlist attached at that index)
- [x] Playlist edit (`e`): transactional buffer — insert-at-point (a track, or another playlist's entries, from the browser), reorder, remove; `Enter` commits (write `.rpl` + sync a loaded deck), `Esc` aborts; commit/abort banner
- [x] Tag edit (`e` on a track): render tag editing in the panel with reflow, replacing the full-screen modal; `Enter` saves, `Esc` cancels
- [x] Distinct colours for the preview / browse / edit states

Resolution surfacing:

- [~] Present the engine's needs-confirmation outcome (ranked candidates) — spun out to [[playlist-needs-confirmation]]


## Log

- Loads are async (decode on a thread), so playlist state can't be attached at open time: `PendingLoad` carries an `attach_playlist`, applied when the deck finishes building.
- `service_deck_frame` can't start a load (no access to `pending_loads`), so auto-advance is a one-shot `advance_requested` flag it sets at end-of-track, acted on by a main-loop pass that resolves the next entry and starts the load.
- Playlist opens reuse the existing browser load path (`apply_browser_load` + the playing-deck confirmation) rather than a parallel flow.
- Add-track mechanism revised (agreed with user): a playlist is built by browsing and appending (`a`), not by loading, since loading detaches the playlist. Create (`n`) attaches an empty playlist to the target deck, which must have a track loaded to host it (an empty deck can't hold an `ActivePlaylist`); otherwise the `.rpl` is created but not attached.
- Browser gained a lightweight `name_prompt` input state for the new-playlist name, rather than a full `BrowserMode`.
- Tasks 1–6 and 8 complete and compiling; overlay (7), needs-confirmation UI (9), and workspace heal/nudge (10) remain.
- Redesigned per Feedback: built the split editor (`PlaylistEditor` over the browser). Editor availability is cached and recomputed on structural edits, not per frame. Live sync re-points the playing index to the same track by content hash after a reorder/remove, so audio is never interrupted. Old `a`/`n`-attach wiring removed. Playback half unchanged. Editing tasks done; needs-confirmation UI and workspace heal/nudge remain.
- Fix: with the playlist half focused, unhandled keys (notably Enter) fell through to the browser and triggered a selection. The playlist half now swallows all keys it doesn't act on; the browser half still passes unhandled keys through for navigation.
- Reorder also bound to `K`/`J`, not only Shift+↑/↓ (terminals don't reliably report Shift+arrow).
- The editor renders in the normal browser region (not fullscreen), split 50/50; fullscreen only when that region is too short.
- Added (user-requested, beyond the original plan): manual playlist skip on the selected deck — `playlist_next` (`alt+n`) / `playlist_prev` (`alt+p`) jump to the next/previous resolvable entry.
- Editor navigation reworked to vim keys (user request): j/k move, h/l switch panels, Backspace up-dir, Enter/a add, K/J reorder, x remove. Letter keys stay text input while a browser search is being typed.
- Normal browser: dropped `l` (load) and `h` (up-dir) aliases to avoid the inconsistency with the editor's h/l panel-switch and accidental loads — `Enter` loads, `Backspace`/`Left` go up.
- Hover-preview (user request): the RHS pane now follows the browser cursor — hovering a `.rpl` opens its editor, moving off closes it; no `e`-to-open. A per-frame reconcile reopens the editor only when the hovered path changes (availability resolve stays off the hot path). `e` now focuses the playlist pane; Esc backs out of it to the browser. `n` highlights the new file so the preview opens it.
- The browser half dims (post-render cell pass) while the playlist pane holds focus, so the active side is obvious.
- Fix: the first hover-preview cut broke adding — moving the cursor to a track to add it closed the pane. Now the pane opens on hovering a playlist and **stays open** while you browse to tracks (so `a`/Enter can append); it switches only on hovering a different playlist, and closes on Esc or browser-close. Esc-dismiss remembers the path so it doesn't instantly reopen, and forgets it once you move off so re-hovering reopens.
- Panel refinements (user feedback): Helix-style insert — `a` after cursor (append), `A` before (like paste p/P); display order `title - artist` (matches the filename convention); hint text moved below the frame, wrapping, naming the keys; in Edit-Browser the panel border dims (browser bright + undimmed) so the active side reads clearly; Browse hint says "load" not "play"; commit/abort return the browser to the edited playlist (commit lands in Browse, ready to load).
- Redesign (pass 2): the hover editor is torn out for a **permanent 70/30 context panel** — a `Panel` state machine (`Preview` / `Browse` / `Edit{focus}`). Preview mirrors the browser highlight (track metadata or playlist contents), recomputed only on highlight change. Browse focuses a playlist read-only (`Enter` plays an entry on the target deck at that index). Edit is a transactional buffer written only on commit, so abort just drops it (`original` snapshot proved unnecessary). Insert-at-point splices a track or another playlist's entries at the cursor. The input block is a `consumed`/`transition` state machine to sidestep the move-out-of-`&mut panel` borrow. Track tag-editing still uses the modal (the in-panel tag editor remains).


## Feedback

- **Status:** partially implemented — playback ships; the editing UI is being redesigned (second pass).
- **Notes (pass 1):** the built create/append flow felt unnatural; replaced with a hover-driven side-by-side editor (browse-to-add, remove/reorder, ▶/⇢ markers, live sync).
- **Notes (pass 2):** the hover editor also didn't fit. Redesigning as a **permanent ~70/30 context panel** with three states — Preview (metadata / playlist contents), Browse (`l`/Enter: read-only, `Enter` plays an entry on the target deck), Edit (`e`: transactional, `Enter` commits / `Esc` aborts). Adds insert-at-point (splice a track or another playlist), moves tag editing into the panel, and makes playlist edits transactional. Supersedes the built hover open/close, append-only, tag modal, and live-while-editing sync. Playback, engine helpers, manual skip, and workspace heal all carry over.
- **Documentation impact:** `keybindings.md` (browser + panel keys); the tag-editor modal keys.


## Conclusion

Completed and shipped as v0.14.19. Playlist support: opening/playing playlists, auto-advance, manual skip, per-deck state and position badge, workspace-mirrored resolution with heal and a no-workspace nudge, and a permanent ~70/30 browser context panel with three states — Preview (track metadata / playlist contents), Browse (Enter loads any entry onto the target deck at that index), and transactional Edit (Helix-style `a`/`A` insert-at-point splicing tracks or whole playlists, `K`/`J` reorder, `x` remove, commit/abort, jump-back-to-playlist) — plus tag editing moved into the panel (the full-screen modal now serves only the load-time rename offer).

The design was reworked twice under testing (append flow → hover editor → permanent panel); the Feedback records the path. The needs-confirmation candidate picker and the outstanding cosmetic polish are spun out to [[playlist-needs-confirmation]].

Documentation impact — map catch-up deferred (not done here): the map has no playlist concept. Next session should add a **Playlist** node (or subtree) under Application, and revisit the **Browser**, **Keymap**, and **Metadata Editor** nodes — the context panel subsumed the tag modal and introduced the browse/edit key layers. `keybindings.md` also needs the panel keys.
