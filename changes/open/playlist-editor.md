# Playlist Editor

**Mode:** Formal

## Intent

The Deck-side of playlist support: reading and writing `.rpl` files, resolving entries to tracks, and the whole operator experience — opening playlists from the browser, per-deck playlist state, an overlay for viewing and editing (remove, reorder), adding the loaded track, creating a new playlist, a position indicator, and auto-advance between tracks. File resolution uses Deck's `@` workspace as the library root, and the descriptive-fallback confirmation is presented to the operator.

Uses [[content-identity-hashing]] for track identity — that is the only shared code. The `.rpl` format, resolution, tags refresh, and resilient writes are Deck's own implementations of the prose spec, organised into a `src/playlist/` module for testability.


## Approach

### `.rpl` format, resolution, and resilient writes from the spec

A `src/playlist/` module implements `playlist.md`'s prose: parse/serialise the JSON schema; file resolution (path-hint confirm, library search with duration/size pre-filter then hash confirm via the shared hasher, descriptive-fallback candidate ranking, unavailable); tags refresh on locate; and resilient writes (validate by re-parse, temp file in the same directory, `.bak1`–`.bak3` rotation, atomic rename, backup recovery). Deck's own code, unit-tested here — not shared with the C player, which implements the same prose independently.

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

When a deck's track finishes (remaining time ≤ 0, not paused) and its active playlist has a next entry, a load is triggered automatically. Each entry is resolved lazily — just before it plays — and hints are updated in the file on a successful relocate.

### File resolution uses `@` workspace as library root, confirmation in-app

The existing workspace (`BrowserState.workspace`, persisted in cache) is passed to the core's resolution as the search root. If no workspace is set, resolution falls back to hint-only (no library search); the existing workspace prompt (`@`) serves this purpose. When the core returns a needs-confirmation outcome (descriptive fallback), Deck presents the ranked candidates for the operator to confirm or reject.


## Unresolved

- **Playlist overlay placement** — full-screen (like the browser) is assumed here, but a split view (playlist panel below the decks) is an alternative. Which fits better with how you expect to use it during a mix?
- **New playlist creation entry point** — browser key is assumed. Is there a case for creating one directly from the player (without opening the browser first)?
- **Multi-deck add-track ambiguity** — "add track to playlist" targets the selected deck's playlist. Is that always the right behaviour, or should it be possible to add to a non-selected deck's playlist?
