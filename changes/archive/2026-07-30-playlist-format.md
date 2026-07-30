# Playlist Format

**Mode:** Formal

## Intent

Deck's own implementation of the resilient playlist format — everything in the format map except the content hash, which the `resilient-playlists` crate already provides. A `.rpl` engine: the entry/playlist data model, JSON read/write, file resolution (locate a track, rewrite hints, refresh tags, rank descriptive candidates), method-version handling, and resilient writes with backups. Pure logic, no UI — the foundation the playlist editor ([[playlist-editor]]) wires into the operator experience.

Not shared with the embedded C player: that implements the same map prose independently. Only the hash is shared (the crate). This is Deck's parallel implementation of the rest.


## Approach

### A `src/playlist/` module depending on the crate

Deck's from-prose implementation lives in a new `src/playlist/` module, using `resilient-playlists` for identity hashing. It is Deck's code, unit-tested here — deliberately separate from the crate, which stays UI- and Deck-free.

### Data model mirrors the map's four-role entry

Serde structs for the entry (identity / description / hints / settings) and the playlist (version + ordered entries), parsing tolerant of unknown fields and preserving the `version`. `settings` is reserved/empty per the map. `identity` carries `hash_algorithm` and `payload_extraction_version` alongside the hash and duration.

### Resolution takes injected library and tag access

File resolution needs to enumerate the library and read a file's tags — both Deck capabilities. They are injected (a library-lister and a tag-reader interface) so the module unit-tests against fakes and stays Deck-independent; Deck supplies the real ones later. This is the same isolation the crate used.

### Resolution is read-only and returns an outcome

Resolution reads via the injected interfaces and returns an outcome — located (no change), relocated (with rewritten hints and any tags refresh), needs-confirmation (ranked descriptive candidates), or unavailable — plus any updated entry. It never writes; the caller persists through the resilient-write path. This keeps resolution pure given its inputs, and lets the descriptive-fallback confirmation (which mutates identity) be a separate, user-gated step.

### Version stamping now; full forward-heal when a second version exists

New and relocated entries are stamped with the crate's current `payload_extraction_version`. On resolve, an entry whose version the crate cannot reproduce is treated as unresolvable-by-hash (falling to descriptive fallback), per the map. The complete forward-heal — retaining superseded extraction rules to confirm-then-rewrite — is deferred until a second version actually exists to migrate from; implementing it now would be untestable speculative code. What ships is the version stamping, the version check, and the graceful-degradation path.

### Resilient writes, self-contained

Validate-by-reparse, temp file in the playlist's directory, `.bak1`–`.bak3` rotation, atomic rename, and recovery from a backup when the primary won't parse — touching only the `.rpl` file and its hidden backup siblings, so it is testable in a temp directory with no Deck types.


## Plan

- [x] Add a path dependency on `resilient-playlists`; create the `src/playlist/` module.
- [x] Entry and playlist data types (serde, unknown-field tolerant, `version` preserved).
- [x] `.rpl` parse and serialise.
- [x] Construct an entry from a track's parts: crate hash plus supplied duration, tags, and file size.
- [x] Library-lister and tag-reader injection interfaces, with test fakes.
- [x] Resolution returning an outcome — path-hint confirm, library search (duration/size pre-filter then hash confirm), descriptive-candidate ranking, unavailable — with hints rewrite and tags refresh in the returned entry.
- [x] Descriptive-fallback adoption: rewrite identity to a chosen candidate.
- [x] Version stamping on new and relocated entries; unresolvable-by-hash for a version the crate can't reproduce.
- [x] Resilient writes: validate-by-reparse, same-directory temp file, `.bak1`–`.bak3` rotation, atomic rename, recovery from a backup on a corrupt primary.
- [x] Bump Cargo patch (0.11.21 → 0.11.22).


## Log

- Built as `src/playlist/mod.rs` in one pass; all ten tasks landed, 9 module tests pass, workspace warning-clean (module carries `#![allow(dead_code)]` until the editor consumes it).
- Resolution is a single `resolve(entry, playlist_dir, &dyn Library) -> Resolution` returning `Found { path, updated_entry: Option<Entry> }` / `NeedsConfirmation { candidates }` / `Unavailable`. `Found` folds steps 1 and 2 plus tags-refresh: `updated_entry` is `Some` only when hints or description actually changed, so the caller persists exactly when there's a change.
- `Library` trait injects the four things resolution needs: `candidates`, `cheap_probe` (duration+size, no decode, for the pre-filter), `read_bytes` (for hash confirm), `read_description` (tags). Deck supplies these over its `@` workspace in the editor change; tests use an in-memory `FakeLibrary`.
- Descriptive-candidate ranking is a placeholder: count of matching non-empty description fields (map leaves ranking implementation-defined). Adequate now; the editor may want something better once real libraries are in play.
- `relative_to` computes `../`-style relative paths between absolute paths (common-prefix walk); falls back to the target's own string when the paths share no root.


## Conclusion

Completed at 0.11.22 as `src/playlist/mod.rs` — the `.rpl` engine on the `resilient-playlists` crate: four-role data model with serde round-trip and unknown-field tolerance, entry construction, `resolve` returning Found / NeedsConfirmation / Unavailable (hint-confirm, pre-filtered library search, tags refresh, descriptive ranking, version-mismatch degradation), descriptive-candidate adoption, and resilient writes with backup rotation and corrupt-primary recovery. Nine unit tests are the deliverable — the module is an invisible engine, so there is no in-app change; it carries `#![allow(dead_code)]` until the editor consumes it.

As planned, method migration ships as version stamping plus graceful degradation only; the full forward-heal waits for a second extraction version to exist. Two items carried to [[playlist-editor]]: the real `Library` over the `@` workspace, and the no-workspace nudge with auto-heal on workspace set (folded into that change's Approach). Descriptive ranking is a placeholder (field-match count) the editor may refine.
