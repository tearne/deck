# resilient-playlists

The portable core of the **resilient playlist** format: the reference implementation of the track content-identity hash, and the conformance corpus that any implementation validates against.

- **`map.md`** — the complete format specification. Portable and self-contained; an implementation in any language can be built from it.
- **`src/lib.rs`** — the reference hasher. Given an audio file, it computes the content identity: a Blake3 hash over the encoded audio payload, with tag regions and container overhead excluded, so identity survives moves, renames, and retags. Supports FLAC, WAV, OGG Vorbis, Opus, MP3, AAC, and M4A.
- **`corpus/`** — the conformance test vectors: small synthetic audio files plus `target_results.json`, the expected hash and audio byte-range for each. An implementation conforms when it reproduces every target result. `corpus/generate.sh` documents how the clean files were made.

## The only thing that must match exactly

Across implementations, the identity **hash** is the one value that has to agree byte-for-byte — a hash over the wrong bytes silently fails to match everyone else. Everything else in `map.md` (the `.rpl` file format, file resolution, resilient writes) is behaviour each implementation performs locally and need not be shared code. So this crate provides the hash and the corpus; a consumer builds the rest from `map.md`.

## Using it

```rust
let bytes = std::fs::read("track.flac")?;
let hash = resilient_playlists::content_hash(&bytes)?;
```

## Validating another implementation

Run your hasher over every file in `corpus/` and compare against `corpus/target_results.json`. Matching every entry — including the tag-placement edge cases, which share a hash with their untagged counterpart — is the definition of conformance. See the Conformance section of `map.md`.

## Tests

```
cargo test -p resilient-playlists
```
