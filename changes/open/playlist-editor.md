# Playlist Editor

**Mode:** Formal

## Intent

The operator experience of playlist support, built on the playlist engine ([[playlist-format]]): opening playlists from the browser, per-deck playlist state, an overlay for viewing and editing (remove, reorder), adding the loaded track, creating a new playlist, a position indicator, and auto-advance between tracks. File resolution uses Deck's `@` workspace as the library root, and the descriptive-fallback confirmation is presented to the operator.

UI and wiring only — the `.rpl` engine (format, resolution, migration, resilient writes) is [[playlist-format]], which in turn uses the `resilient-playlists` crate for identity. This change supplies the real library-lister and tag-reader the engine's resolution needs, and presents its outcomes.


## Approach

### Browser treats `.rpl` files as selectable

Add a `BrowserEntry::Playlist` variant. `.rpl` files are highlighted in the browser listing (parallel to audio files) and selectable. Selecting one opens a playlist on the target deck and loads the first resolvable track. Non-resolvable entries are skipped to the next.

### Deck carries optional playlist state

Add `playlist: Option<ActivePlaylist>` to the `Deck` struct, where `ActivePlaylist` holds the loaded `Playlist` and the current index. All three decks can independently hold playlists. When a deck is cleared or a standalone track is loaded, its playlist state is dropped.

### Playlist overlay for viewing and editing

A new full-screen overlay (parallel to the browser, occupying the same screen area) shows the active playlist for the selected deck. Entries are listed with description and resolution status (found / unavailable). The current position is highlighted. Keys within the overlay handle remove and reorder. The overlay is opened by a configurable action key when a playlist is active; opening it when no playlist is active is a no-op.

### Adding tracks to a playlist

From the player (not the overlay), a key appends the selected deck's currently loaded track to that deck's active playlist. If the deck has no active playlist, the action is a no-op (a separate flow creates one first).

### Creating a new playlist

From the browser, a dedicated key prompts for a playlist name and creates an empty `.rpl` file in the current browser directory. The new playlist is immediately opened on the target deck, ready to receive tracks via the add-track action above.

### Visual indicator in the deck info bar

When a deck has an active playlist, the current position `x / y` is shown in the info bar alongside the track name. A short prefix label distinguishes playlist mode from standalone loading.

### Auto-advance hooks into `service_deck_frame`

When a deck's track finishes (remaining time ≤ 0, not paused) and its active playlist has a next entry, a load is triggered automatically. Each entry is resolved lazily — just before it plays, via the engine — and the file is rewritten on a successful relocate.

### File resolution uses `@` workspace as library root, confirmation in-app

The existing workspace (`BrowserState.workspace`, persisted in cache) becomes the engine's library root — Deck's library-lister enumerates it. If no workspace is set, resolution falls back to hint-only (no library search); the existing workspace prompt (`@`) serves this purpose. When the engine returns a needs-confirmation outcome (descriptive fallback), Deck presents the ranked candidates for the operator to confirm or reject.

### No-workspace nudge and auto-heal on workspace set

When an open playlist has unavailable entries and no workspace is set, surface a nudge that setting a workspace (`@`) will relocate moved tracks — so the operator understands why entries are dark and how to fix it. Setting or changing the workspace then re-resolves the open playlists' currently-unavailable entries (found entries need nothing), updates their displayed status, and persists rewritten hints for any that relocate — the payoff is immediate. Re-resolution scans and hashes candidate files, so for a large library it should heal only the unavailable entries and avoid blocking the UI (a brief "relocating…" indication, or off-thread). The engine already supports this: re-resolving is calling `resolve` again with a now-populated library, taking the returned updated entry to persist.


## Unresolved

- **Playlist overlay placement** — full-screen (like the browser) is assumed here, but a split view (playlist panel below the decks) is an alternative. Which fits better with how you expect to use it during a mix?
- **New playlist creation entry point** — browser key is assumed. Is there a case for creating one directly from the player (without opening the browser first)?
- **Multi-deck add-track ambiguity** — "add track to playlist" targets the selected deck's playlist. Is that always the right behaviour, or should it be possible to add to a non-selected deck's playlist?
