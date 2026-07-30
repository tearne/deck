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
│ └ Conformance
├ File Resolution
│ ├ Descriptive Fallback
│ ├ Tags Refresh
│ └ Method Migration
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
  "hash_algorithm":           "blake3",
  "payload_extraction_version": 1,
  "content_hash":             "<hex>",
  "duration_secs":            214.7
}
```

- `duration_secs` lives here rather than in hints because it is content-derived and participates in candidate pre-filtering during resolution.
- Two independent axes describe how `content_hash` was produced. `hash_algorithm` names the hash function; `payload_extraction_version` names the byte-range rules that selected the bytes fed into it. Either can change without the other — a new hash function, or a correction to which bytes count as audio — so they are versioned separately. Both are recorded per entry, so a partially-migrated playlist (some files present and re-hashed, some missing and not) can hold a mix.
- An implementation encountering a value it does not implement — an unknown `hash_algorithm`, or a `payload_extraction_version` it cannot reproduce — must treat the entry as unresolvable by hash rather than computing a wrong result silently. See [Method Migration](#method-migration) for how an entry made by an older version is healed forward.

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
[Down](#conformance)

Identity is a hash of the file's **encoded audio payload** — the raw compressed bytes on disk, tag regions excluded, never decompressed to PCM. Any implementation reading the same file computes the same hash byte-for-byte.

Two fields record how the hash was made: `hash_algorithm` (the hash function, currently `"blake3"`) and `payload_extraction_version` (the byte-range rules that chose the input bytes, currently `1`). They are versioned separately because either can change alone. An implementation encountering a value it cannot reproduce must treat the entry as unresolvable by hash rather than computing a wrong result silently; a bump to the extraction rules is healed forward per [Method Migration](#method-migration).

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
- [Conformance](#conformance) — how an implementation proves its hashing is correct


# Conformance

[Up](#track-identity)

Byte-range extraction is the one place implementations must agree exactly — a hash computed over the wrong bytes silently fails to match every other implementation. Prose alone can't guarantee that agreement, so conformance is defined by a shared **test-vector corpus**: a set of small audio files with, for each, its expected content hash and audio byte-range. An implementation conforms when it reproduces every expected hash.

The corpus covers one clean file per supported format, plus tag-placement edge cases — a file with a prepended ID3v2 tag, a file wrapped in ID3v1 and APEv2 trailers, and so on. These edge cases carry the *same* expected hash as their untagged counterpart: identical hashes prove the tag regions are excluded, which is where implementations most easily diverge.

**Detail**

- The corpus files are synthetic (generated from a tone, no third-party audio) and pinned alongside `target_results.json` — the expected hashes and byte-ranges — so the contract is immutable and reproducible offline.
- Reference implementation: the `resilient-playlists` crate (`src/lib.rs`), with its corpus and `target_results.json` under `corpus/`. A new implementation validates against those target results.


# File Resolution

[Up](#resilient-playlists)
[Down](#descriptive-fallback)
[Down](#tags-refresh)
[Down](#method-migration)

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

On confirmed re-link: rewrite hints *and* overwrite `identity` (`hash_algorithm`, `payload_extraction_version`, `content_hash`, and `duration_secs`, all from the new file computed with the implementation's current method) and refresh `description` from its tags. This is the only sanctioned mutation of identity.

No match and no confirmed re-link → the entry is kept and shown unavailable.

> [!IMPORTANT] Explicit user confirmation is required before adoption. A wrong silent re-link is worse than a missing track.

**Detail**

- Candidates are ranked by implementation-defined description similarity.
- The fallback also re-links a shared playlist against a friend's differently-encoded rip of the same track, provided the user confirms.


# Tags Refresh

[Up](#file-resolution)

Whenever a file is located (steps 1–2 of [File Resolution](#file-resolution)), compare its current tags against the stored `description`. If any field differs, refresh `description` from the file.

A plain move leaves tags unchanged so nothing refreshes; deliberate retags propagate automatically.


# Method Migration

[Up](#file-resolution)

A correction to the byte-range rules bumps `payload_extraction_version`, which changes the hashes a newer implementation computes. Without care, every entry made by the older version would fail its hash confirm even for files sitting exactly where the playlist says — and degrade to the descriptive fallback needlessly. Migration heals this forward, one entry at a time.

When resolving an entry whose `payload_extraction_version` is older than the one the implementation now produces, it confirms the file using the entry's *stated* (older) version — so an implementation must retain the older extraction rules — and on a match, rewrites the entry's `content_hash` and `payload_extraction_version` to the current values. The same applies to `hash_algorithm`.

> [!IMPORTANT] Migration is per entry and opportunistic: an entry heals only when its file is found. A missing file keeps its older version until it reappears, so a playlist may legitimately hold entries at different versions.

**See also**

- [Identity](#identity) — the two version fields and why they are per entry


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
