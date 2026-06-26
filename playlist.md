# Resilient Playlists

[Down](#entry-structure)
[Down](#track-identity)
[Down](#file-resolution)
[Down](#resilient-writes)

```
Resilient Playlists
├ Entry Structure
│ ├ Identity
│ ├ Description
│ ├ Hints
│ └ Settings
├ Track Identity
├ File Resolution
│ ├ Descriptive Fallback
│ └ Tags Refresh
└ Resilient Writes
  ├ Write Procedure
  └ Backup Scheme
```

An ordered list of tracks saved as a `.rpl` (**r**esilient **p**lay**l**ist) file, identified by audio content with file location as a hint. When a track moves, the playlist finds it and quietly updates its own hints — accurate through library reorganisation with no manual repair.

Designed to be portable: the format and algorithms here are a complete specification for any conforming implementation. Deck-specific behaviour (browser UX, key bindings, which directory to search) is left to the implementation.

**Detail**

```json
{
  "version": 1,
  "entries": [ ... ]
}
```

- The filename stem is the playlist name; the file may live anywhere the user places it.
- The `version` field allows forward extension without breaking older readers.


# Entry Structure

[Up](#resilient-playlists)
[Down](#identity)
[Down](#description)
[Down](#hints)
[Down](#settings)

Each entry is a structured object with four visibly distinct roles: what the track *is*, what it is *called*, where it was *last seen*, and how it should *play*. Keeping these roles separate means the entry remains displayable when the file can't be found, and rewritable hints don't risk corrupting the things that matter.

The same track may appear more than once in a playlist with different settings — two entries sharing an identity but playing different sections are both valid.

**Detail**

```json
{
  "identity":    { ... },
  "description": { ... },
  "hints":       { ... },
  "settings":    { ... }
}
```

**See also**

- [File Resolution](#file-resolution) — when hints are rewritten; when identity is mutated


# Identity

[Up](#entry-structure)

The content-derived, unambiguous identifier for the track — stable across moves, renames, and retags. Immutable except on a confirmed descriptive re-link (see [File Resolution](#file-resolution)).

**Detail**

```json
"identity": {
  "hash_algorithm": "blake3",
  "content_hash":   "<hex>",
  "duration_secs":  214.7
}
```

- `duration_secs` lives here rather than in hints because it is content-derived and participates in candidate pre-filtering during resolution.
- `hash_algorithm` records which algorithm produced `content_hash`, so a future spec version can introduce a new algorithm without breaking existing entries. An implementation encountering an unknown value must treat the entry as unresolvable by hash rather than computing a wrong result silently.

**See also**

- [Track Identity](#track-identity) — how `content_hash` is computed
- [File Resolution](#file-resolution) — the one sanctioned mutation of identity


# Description

[Up](#entry-structure)

A durable record of the track's metadata, sourced from the file's tags but held independently. The entry displays correctly when the file can't be found, and a shared playlist tells a friend exactly what to look for.

**Detail**

```json
"description": {
  "artist": "Artist Name",
  "title":  "Track Title",
  "album":  "Album Name",
  "year":   "2023"
}
```

- Only `artist`, `title`, `album`, `year` are stored — not a verbatim tag dump.
- Refreshed automatically when a located file's tags differ (see [Tags Refresh](#tags-refresh)).

**See also**

- [File Resolution](#file-resolution) — when description is refreshed


# Hints

[Up](#entry-structure)

Where the file was last seen. Rewritable and fully re-derivable — losing them costs only a re-search, never any meaningful data.

**Detail**

```json
"hints": {
  "relative_path":   "../Music/Artist Name - Track Title.flac",
  "file_size_bytes": 28451920
}
```

- `relative_path` is relative to the directory containing the `.rpl` file. Arbitrary relative paths including `../` are permitted — the path is only a hint, confirmed by hash before use, so a wrong path falls through to search rather than loading the wrong audio.
- `file_size_bytes` is used as a cheap pre-filter during relocation search (see [File Resolution](#file-resolution)).

**See also**

- [File Resolution](#file-resolution) — when hints are rewritten


# Settings

[Up](#entry-structure)

Per-entry playback configuration. Currently reserved and always empty; the `version` field at the playlist level gates future additions.

Likely future content: in/out points (to play a sub-section of a track), per-entry gain, and a `transition_out` object for continuous playback (crossfade duration and similar).

**Detail**

```json
"settings": {}
```


# Track Identity

[Up](#resilient-playlists)

Identity is a hash of the file's **encoded audio payload** — the raw compressed bytes on disk, tag regions excluded, never decompressed to PCM. Any implementation reading the same file computes the same hash byte-for-byte.

The `hash_algorithm` field records which algorithm was used, so a future version of the spec can introduce a different algorithm without breaking existing entries. The current value is `"blake3"`. An implementation encountering an unknown value must treat the entry as unresolvable by hash rather than computing a wrong result silently.

The cost: re-encoding a track (different compression settings, format conversion) yields a new identity. This case is handled by the descriptive fallback in [File Resolution](#file-resolution).

**Detail**

Per-container audio payload byte ranges — all other ranges are tags or container overhead and must be excluded:

| Format       | Audio payload                                                                                                    |
|--------------|------------------------------------------------------------------------------------------------------------------|
| FLAC         | All `FRAME` blocks; excludes the `METADATA_BLOCK` chain at file start                                           |
| MP3          | All MPEG audio frames (sync word `0xFF 0xE*` or `0xFF 0xF*`); excludes leading ID3v2, trailing ID3v1 and APEv2 |
| OGG Vorbis   | All Ogg pages after the three header pages (identification, comment, setup)                                      |
| WAV          | The `data` chunk body only; excludes `fmt `, `LIST`, and all other chunks                                       |
| AAC          | All ADTS frames; excludes any leading ID3v2 header                                                              |
| OPUS         | All Ogg pages after the two header pages (identification, comment)                                               |
| M4A          | The `mdat` box body only; excludes `ftyp`, `moov`, `free`, and all other boxes                                  |

Implementations must iterate each file's structure to extract these ranges exactly — byte-range precision is required for hash interoperability between implementations.

**See also**

- [File Resolution](#file-resolution) — how the content hash is used in candidate screening


# File Resolution

[Up](#resilient-playlists)
[Down](#descriptive-fallback)
[Down](#tags-refresh)

How an implementation locates the audio file for a given entry and keeps location hints up to date.

1. Resolve `hints.relative_path` against the playlist's directory. If a file exists there, confirm by content hash. Match → done; update nothing.
2. If miss, search the implementation's library root. Screen candidates cheapest-first: `duration_secs` within tolerance and `file_size_bytes` within 1% (no decode) → content hash confirm. First match wins → rewrite hints.
3. If no hash match anywhere, see [Descriptive Fallback](#descriptive-fallback).
4. If no match and no confirmed re-link, keep the entry and mark it unavailable. Never silently drop; never silently re-link.

**Detail**

- Duration tolerance: **±2 seconds**.
- File-size tolerance: 1% — accommodates minor container rewrites that don't change the audio payload.

**See also**

- [Track Identity](#track-identity) — how the content hash is computed
- [Entry Structure](#entry-structure) — which fields are rewritten at each step


# Descriptive Fallback

[Up](#file-resolution)

When no hash match exists anywhere, the track may have been re-encoded — its compressed bytes are entirely new so its identity can never match. The fallback offers library files whose duration and description are similar, for the user to confirm.

On confirmed re-link: rewrite hints *and* overwrite `identity` (`hash_algorithm`, `content_hash`, and `duration_secs` from the new file) and refresh `description` from its tags. This is the only sanctioned mutation of identity.

No match and no confirmed re-link → the entry is kept and shown unavailable.

> [!IMPORTANT] Explicit user confirmation is required before adoption. A wrong silent re-link is worse than a missing track.

**Detail**

- Candidates are ranked by implementation-defined description similarity.
- The fallback also re-links a shared playlist against a friend's differently-encoded rip of the same track, provided the user confirms.


# Tags Refresh

[Up](#file-resolution)

Whenever a file is located (steps 1–2 of [File Resolution](#file-resolution)), compare its current tags against the stored `description`. If any field differs, refresh `description` from the file.

A plain move leaves tags unchanged so nothing refreshes; deliberate retags propagate automatically.


# Resilient Writes

[Up](#resilient-playlists)
[Down](#write-procedure)
[Down](#backup-scheme)

Writes protect identity, entry order, and the last-known description against interrupted writes and parser bugs — description refreshes from tags when the file is found, but is the only record when it isn't. Location hints are disposable: if lost, file resolution will recover them.

> [!IMPORTANT] Identity and entry order are irreplaceable; description is the only record of a track's metadata when the file is absent. Location hints are disposable and will be re-derived by file resolution.


# Write Procedure

[Up](#resilient-writes)

Required behaviour for every write to a `.rpl` file:

1. Serialise to JSON and validate that the result re-parses cleanly. If validation fails, abort — no partial write.
2. Write to a temporary file in the **same directory** as the `.rpl` file.
3. Rotate the backup set: shift `.bak2` → `.bak3` (drop oldest), `.bak1` → `.bak2`, current primary → `.bak1`.
4. Rename the temporary file to the primary filename.

**Detail**

- The temporary file must be in the same directory as the playlist to guarantee the rename is atomic (same filesystem — cross-filesystem moves are not atomic on most systems).

**See also**

- [Backup Scheme](#backup-scheme) — the naming and rotation scheme used in step 3


# Backup Scheme

[Up](#resilient-writes)

Up to **3** hidden sibling backups per playlist, named `.<stem>.rpl.bak1`, `.<stem>.rpl.bak2`, `.<stem>.rpl.bak3`. Hidden naming (leading `.`) keeps them out of directory listings that filter for `.rpl`.

If the primary `.rpl` file fails to parse on load, attempt each backup in slot order (`.bak1` first) and use the first that parses. Surface the fallback to the user — silent recovery is acceptable; silent recovery with no indication is not.
