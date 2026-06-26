# Playlist Specification

**Mode:** Explore

## Intent

`deck` has no way to save and recall an ordered set of tracks. Add playlists, with one hard requirement: a saved playlist must keep pointing at the right audio after the library is reorganised — files renamed, moved, or retagged. It does this by identifying each track by its audio content rather than its location, and it is self-healing: when a track turns up in a new place, the playlist quietly updates itself, so repeated use keeps it fast rather than letting it rot.

The design must also suit the embedded devices and SD-card filesystems this is meant to run on: no always-synced library index, nothing on disk to go stale.

This change delivers only the **specification** — a portable format definition precise enough that two independent programs (Deck, and a separate embedded player used for testing) produce and consume identical playlists. Deck's editing, playback, and resolution wiring, and the embedded implementation, follow as separate changes.


## Approach

### The deliverable is a standalone map-style specification

A new `playlist.md` at the project root — a map document using the same node/navigation format as `map.md`, so it reads consistently with the application map and can serve as a portable spec for other implementations. The main `map.md` gains a `Playlist` entry in the Application tree with a cross-file link. The spec pins formats and behaviour, not Deck UI — anything Deck-specific (browser, key bindings, decks) is named only as "the implementation" and deferred.

### File format

Extension `.rpl` (resilient playlist). Human-readable JSON with a top-level **version** field. The filename is the playlist's name; the file may live anywhere the user puts it.

### Entry structure separates identity, description, and hints

Each entry is a structured object — not a flat dictionary — with three roles kept visibly distinct:

- **Identity** (immutable, content-derived): the content hash, the probe hash (a fixed leading span), and duration. Defines what the track is; rewritten only by a confirmed descriptive re-link (see Resolution).
- **Description** (durable record): artist, title, album, year. Sourced from the file's tags but held as a durable record — not a verbatim tag mirror — so a track can be re-purchased, or a shared playlist tells a friend exactly what to buy, and so the entry still displays when its file can't be found. (A display label is the implementation's to derive; not stored.)
- **Location hints** (rewritable, re-derivable): last-known path **relative to the playlist file**, and file size. Self-heal overwrites only these; losing them costs just a re-search. Arbitrary relative paths (including `../`) are permitted — the path is only a hint, and any file found is confirmed by hash before use, so a wrong path misses and falls through to search rather than loading the wrong audio.

Description and duration also pre-filter relocation candidates; the probe hash screens survivors without full-decoding each.

A reserved per-entry slot is left for future playback/mix settings (sub-section in/out points, per-track gain), with crossfade as a likely per-transition setting; the version field lets these be added without breaking older playlists.

### Track identity

Identity is a Blake3 hash of the file's **encoded audio payload** — the compressed audio bytes as they sit on disk, with tag regions excluded. Nothing is decoded, so any implementation reading the same file computes the same hash byte-for-byte on every format; the cost is that re-encoding a track yields a new identity, handled by the descriptive fallback. The full hash covers the whole payload; the probe hash covers a fixed leading **byte** count for cheap screening. The spec must define, per supported container (flac, mp3, ogg, wav, aac, opus, m4a), exactly which byte ranges are audio payload versus tags.

### Resolution algorithm

1. Resolve the entry's relative path against the playlist's directory; if a file is there, confirm by hash.
2. On a miss (path gone, or hash differs), search an **implementation-defined library root**, screening candidates cheapest-first: duration + file size (no decode) → probe hash → full-hash confirm.
3. Still no hash match anywhere → **descriptive fallback**: a re-encoded copy shares none of its bytes, so its hashes are entirely new and can never match. Offer library files whose duration matches within a tolerance the spec fixes and whose description is similar, ranked, for the user to confirm.
4. On an exact or probe relocate, rewrite the relative path. On a **confirmed** descriptive re-link, also rewrite the entry's identity (both hashes) and refresh its description from the new file — the one sanctioned mutation of the otherwise-immutable identity.
5. No hash match and no confirmed re-link → entry kept and shown unavailable; never silently dropped, never silently re-linked.

The descriptive fallback is inexact by nature, so the spec **requires** explicit user confirmation before adoption — better than nothing when a track has been re-encoded, without risking a wrong silent match. The same mechanism re-links a shared playlist against a friend's differently-encoded rip of the same track.

### Tags refresh on resolve

When a located file's basic tags differ from the stored description, refresh the description from the file. Tag rework is expected and should propagate; a plain move leaves tags unchanged, so nothing refreshes.

### Resilient writes

The format mandates the on-disk conventions every implementation must honour: relative paths, the version field, and a rotating hidden sibling-backup naming scheme (small fixed count) used as the recovery source. The write procedure is a required behaviour: atomic tmp + rename, validate the serialized JSON re-parses before the rename, and on load fall back to the most recent valid backup if the primary won't parse. Identity, description, and order are the precious, non-re-derivable data the backup protects; location hints are disposable.

### Deferred to follow-on changes

Browser listing/opening/searching of `.rpl` files; create-and-name UX (location within the current browser directory, name prompt); the library root being Deck's `@` workspace and the prompt-to-set-one when absent; playback and auto-advance; playlist editing; embedded undo-history.


## Plan

**Topics**

- Root node — playlist concept, `.rpl` format (JSON, version field, filename-as-name, may live anywhere)
- Entry structure — identity / description / hints separation, reserved slot for future per-entry settings
- Track Identity — Blake3 hash of encoded audio payload, probe hash, per-container byte-range table (flac, mp3, ogg, wav, aac, opus, m4a)
- Resolution — 5-step algorithm, descriptive fallback, confirmation requirement, mutation rules
- Tags Refresh — when description is refreshed and what triggers it
- Resilient Writes — backup scheme (rotating hidden siblings), write procedure, load fallback
- Cross-reference — add `Playlist` to the Application tree in `map.md` with a cross-file link

**Done when:** `playlist.md` is a complete, self-contained map document covering all seven topics above, constants (probe byte count, duration tolerance, backup count) are pinned to specific values, and `map.md` references it in the Application tree.


## Conclusion

Completed. `playlist.md` delivered as a standalone map-style specification at the project root. The Log captures all deviations from the Approach; the `map.md` cross-reference deferral is intentional, not a gap — it follows the sync rule and will be added when Deck supports playlists.


## Log

- Probe hash dropped during build — duration + file-size pre-filter judged sufficiently selective; complexity not justified, especially on embedded devices. Resolution steps simplified accordingly.
- `hash_algorithm` field added per-entry to future-proof against algorithm changes; single hash per entry retained (multi-hash approach ruled out as over-engineering).
- `settings` added as a fourth entry role (alongside identity, description, hints); crossfade/continuous playback noted as future `transition_out` in settings rather than a separate transition object.
- Root node renamed "Resilient Playlists"; "Resolution" renamed "File Resolution" for clarity.
- Entry structure split into four child nodes (Identity, Description, Hints, Settings); File Resolution split into Descriptive Fallback and Tags Refresh; Resilient Writes split into Write Procedure and Backup Scheme.
- Write procedure steps corrected — original order was logically wrong (renamed over primary before promoting it to backup); fixed to rotate backups first, then rename temp into place.
- `map.md` cross-reference deferred — map sync rule prohibits adding Playlist to the application map until Deck actually supports playlists.
