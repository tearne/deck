# Content Identity Hashing

**Mode:** Explore

## Intent

The one part of the resilient playlist format that every implementation must reproduce byte-for-byte: the track content-identity hash, computed from a file's encoded audio payload (tags excluded) across all seven supported formats. Deck and a separate C player for embedded devices must compute identical hashes for the same file. This is delivered as a precise reference implementation plus a checked-in test-vector corpus that defines conformance, and the playlist specification gains a section describing that testing approach for other implementors.

Everything else in `playlist.md` — the `.rpl` format, resolution, tags refresh, resilient writes — is behaviour each implementation performs locally from the prose, needing no shared code. Only the hash is fragile and shared, so only it is built here.


## Approach

### Hasher only — the sole shared, fragile piece

The deliverable is a function from an audio file to its spec-conformant identity hash, nothing more. The hash is derived from the file on disk, not from any `.rpl` structure, so this stands entirely alone — no format parsing, no Deck types.

### Synthetic, self-generated corpus — pinned in-repo, not hosted

Corpus files are generated (a fraction of a second of a synthetic tone or noise, encoded to each format), so they carry no third-party copyright and can be committed freely. They are pinned in-repo rather than linked externally: a contract other implementors reproduce must be immutable and available offline, which external hosting can't guarantee. The files are tiny (a few KB each). A generator script records provenance; the committed files plus their expected hashes are the contract.

### Portable extraction; robust formats first

Byte-range extraction avoids Rust-specific idioms so the C port can follow it directly. FLAC, WAV, OGG, and OPUS (container formats with signposted sections) come first to establish the harness; MP3, AAC, and M4A (tag-ambiguous, sync-word-scanned) follow — that is where cross-implementation agreement is hardest.

### Hand-crafted edge cases for tag placement

Beyond one clean file per format, the corpus includes deliberate tag placements — ID3v2 + ID3v1 + APEv2 around MP3 frames, a prepended ID3 on a FLAC, picture blocks, and similar — the cases where implementations most easily diverge. These may be hand-assembled to control the exact payload boundaries.

### The spec carries the conformance approach

`playlist.md` gains a section describing how an implementor validates their hasher against the corpus, and names Deck's module as the reference implementation, so the portable spec stays self-sufficient for a third party.

### The extraction method is versioned, decoupled from the hash algorithm

The byte-range rules carry their own version (`payload_extraction_version`), separate from `hash_algorithm` — either can change without the other. The hasher declares the version it implements as a constant; both fields are recorded per entry (not playlist-wide) so a partially-migrated playlist can hold a mix. The spec defines the forward-healing migration so a future correction to the rules doesn't strand existing playlists. The migration *behaviour* is resolution logic (Change B); this change ships only the constant and the spec.


## Plan

**Topics**

- Encoded-payload byte-range extraction plus Blake3 per format: FLAC, WAV, OGG, OPUS first, then MP3, AAC, M4A.

- Test-vector corpus: a synthetic clean file per format plus tag-placement edge cases, a generator script for provenance, and `target_results.json` (expected hash + byte-range per file), all asserted by tests.

- Spec update: a conformance-testing section in `playlist.md` covering the corpus approach and pointing to the reference implementation.

- Extraction-method versioning: a `payload_extraction_version` constant in the hasher (decoupled from `hash_algorithm`), and the spec's per-entry version fields plus the forward-healing Method Migration behaviour.

- Package as a standalone crate: move the hasher, corpus, and spec into a `resilient-playlists/` workspace member with a README, so the shared kernel is one self-contained, portable unit. The compile boundary (no Deck deps) replaces the `#![allow(dead_code)]` marker.

**Done when** every supported format and edge case hashes to its target result in tests, the spec documents the conformance approach, the extraction method carries a version with the spec's migration story defined, and the kernel lives in its own `resilient-playlists` crate that builds and tests independently of Deck.


## Log

- Module is `src/playlist/mod.rs` (registered in main.rs). API: `content_hash(&[u8]) -> Result<String>` and `payload_ranges(&[u8], AudioFormat)` (ranges exposed so the corpus can assert exact boundaries, not just the hash). Format detected from leading bytes, not the filename. Extraction kept plain for the C port.
- Format progress: WAV done (data-chunk body only), unit-tested with constructed bytes. Remaining: FLAC, OGG, OPUS, then MP3, AAC, M4A.
- Corpus tooling: no encoders in the base env; installed `ffmpeg` (and `flac`) — ffmpeg covers all seven formats, so corpus files will be generated synthetically (sine/noise source) rather than hand-crafted. Location `resources/corpus/` with `manifest.json` (file → format + expected hash + payload range), asserted by a test.
- Harness established end-to-end (WAV + FLAC): detection by magic bytes with a leading-ID3v2 skip helper; corpus files `clean.wav`/`clean.flac` generated by ffmpeg with `-bitexact -map_metadata -1`; `manifest.json` holds the contract; test `corpus_matches_manifest` asserts hash and payload boundaries. FLAC offset independently verified — computed payload start lands exactly on the frame sync `0xFFF8`, and the 8 KB PADDING block is correctly excluded.
- OGG Vorbis + OPUS done: shared Ogg-page parser counts completed packets (a lacing value < 255 ends a packet), skips the codec's header packets (3 Vorbis / 2 Opus), and takes every page from the first audio page to EOF (page headers included — the serial/CRC are part of the file both readers see). Codec detected from the first packet signature (`OpusHead` / `\x01vorbis`). Offsets independently verified against a page dump (ogg 3352, opus 137). Corpus + manifest extended; `corpus_matches_manifest` green across WAV/FLAC/OGG/OPUS.
- Remaining formats implemented: M4A (`mdat` box body, offset verified 44..1247), MP3 and AAC (framed streams: payload between a leading ID3v2 and trailing ID3v1/APEv2, sharing `framed_payload_ranges`). MP3 vs AAC-ADTS disambiguated by the frame-header layer bits (ADTS layer 00, MP3 non-zero). All seven clean formats now pass `corpus_matches_manifest`; build is warning-clean (module carries `#![allow(dead_code)]` until Change B consumes it).
- Corpus recipe: `ffmpeg -y -bitexact -f lavfi -i "sine=frequency=440:duration=0.08:sample_rate=44100" -ac 1 -map_metadata -1 [-c:a <codec>] resources/corpus/clean.<ext>`. For MP3 baseline: `-c:a libmp3lame -write_xing 0 -id3v2_version 0`; AAC: `-c:a aac -f adts`; M4A: `-c:a aac`.
- Edge cases done: `tags_do_not_change_identity` asserts wrapping FLAC/MP3/AAC in synthetic ID3v2 (leading) + ID3v1/APEv2 (trailing) leaves the hash unchanged. Two tagged files committed (`id3-prepended.flac`, `tagged.mp3`) with manifest entries whose hashes match their clean counterparts — the visible proof of tag exclusion. Generated by the ignored `write_tagged_corpus` test.
- Provenance: `resources/corpus/generate.sh` documents the ffmpeg commands for the clean files; the manifest is the pinned contract.
- Spec: `playlist.md` gains a Conformance node (child of Track Identity) describing the corpus approach, the tag-invariance edge cases, and pointing to `src/playlist/` + `resources/corpus/` as the reference implementation. Tree overview and nav links updated.
- All done-when conditions met: seven formats + edge cases hash to their manifest values; the module has no Deck dependencies; the spec documents conformance. Build warning-clean.
- Versioning added to scope (design settled with user): `hash_algorithm` and `payload_extraction_version` decoupled as two independent axes, both recorded per entry (simplest; supports partially-migrated playlists where missing files can't be re-hashed). Hasher gains `PAYLOAD_EXTRACTION_VERSION = 1`. Spec: the new field in Identity, and a Method Migration node (child of File Resolution) defining opportunistic per-entry forward-healing — confirm a found file with its stated older version, then rewrite to current. Migration behaviour itself is Change B.
- Packaged as a standalone crate `resilient-playlists/` (workspace member): `src/lib.rs` (was `src/playlist/mod.rs`, now `pub` API), `corpus/` (was `resources/corpus/`), `map.md` (was `playlist.md` — kept the project's map-format naming), and a new `README.md`. The corpus's `manifest.json` was renamed `target_results.json` (clearer — the results an implementation must reproduce); the test is `corpus_matches_target_results`. Root `Cargo.toml` gained `[workspace] members = ["resilient-playlists"]`; `mod playlist` removed from Deck. The crate has no Deck dependency (boundary now compile-enforced), so the `#![allow(dead_code)]` marker is gone. `cargo test -p resilient-playlists` green; Deck build warning-clean; Deck's 14 tests unaffected. Deck will add a path dependency in Change B when it consumes the hasher.


## Conclusion

Delivered as the standalone `resilient-playlists` crate: a reference hasher for all seven formats (FLAC, WAV, OGG Vorbis, Opus, MP3, AAC, M4A), a conformance corpus (`corpus/` + `target_results.json` + `generate.sh`), the format `map.md`, and a README. Every clean format and the tag-placement edge cases hash to their target results; the tag cases share a hash with their untagged counterpart, proving tag exclusion. Each payload boundary was independently cross-checked (e.g. FLAC landing on the frame sync).

Scope grew during build, by agreement: extraction-method versioning (`payload_extraction_version`, decoupled from `hash_algorithm`, both per entry) with the spec's forward-healing Method Migration; and packaging as a workspace crate so the "no Deck dependencies" boundary is compile-enforced (retiring the `#![allow(dead_code)]` marker).

Deferred to Change B ([[playlist-editor]]): the `.rpl` format I/O, resolution, resilient writes, and all UI — Deck's own from-prose code, using this crate for identity. The migration *behaviour* also lands there. Spun off along the way: [[identity-stability-check]] (before/after hash check on tag edits as a live conformance guard).

Known limit worth carrying forward: automated tests validate against results this implementation produced, so they catch regressions but don't independently prove spec-correctness — the C player agreeing against the same corpus is the real proof, still pending. One untested case noted: an MP3 encoder "info" frame (spec counts it as audio) has no edge-case corpus file yet.
