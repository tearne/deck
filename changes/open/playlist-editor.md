# Playlist Editor

**Mode:** Formal

## Intent

Implement playlist support in Deck, wiring the resilient playlist format (`playlist.md`) into the player. The operator can create, open, edit, and play back playlists from within Deck.

This covers the Deck-specific side deferred from the specification change: browsing and opening `.rpl` files, creating a new playlist at a named location, adding and removing tracks, reordering entries, playback with auto-advance between tracks, and file resolution using Deck's `@` workspace as the library root.

When a deck is playing from a playlist, this is visually indicated and the current position within the list is apparent to the operator.


## Approach

### New `playlist` module

A new `src/playlist/mod.rs` implementing the `.rpl` format: parsing, serialising, the file resolution algorithm, and resilient writes. The module is independent of Deck UI — it handles the format and resolution logic only, so the same code can be tested in isolation and reused by the embedded player.

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

When a deck's track finishes (remaining time ≤ 0, not paused) and its active playlist has a next entry, a load is triggered automatically. Each entry is resolved lazily — just before it plays — using the resolution algorithm, and hints are updated in the file on a successful relocate.

### File resolution uses `@` workspace as library root

The existing workspace (`BrowserState.workspace`, persisted in cache) is passed to the resolution algorithm as the search root. If no workspace is set, resolution falls back to hint-only (no library search); the operator is not prompted automatically, but the existing workspace prompt (`@`) serves this purpose.


## Unresolved

- **Playlist overlay placement** — full-screen (like the browser) is assumed here, but a split view (playlist panel below the decks) is an alternative. Which fits better with how you expect to use it during a mix?
- **New playlist creation entry point** — browser key is assumed. Is there a case for creating one directly from the player (without opening the browser first)?
- **Multi-deck add-track ambiguity** — "add track to playlist" targets the selected deck's playlist. Is that always the right behaviour, or should it be possible to add to a non-selected deck's playlist?
