# Re-key Track Data to Content Identity

**Mode:** Formal

## Intent

Per-track memory (BPM, offset, cue, gain) is keyed by a decoded-PCM hash (`hash_mono`) — computed by decoding the audio to mono samples. Playlists and the tag editor instead key on **content identity** ([[content-identity-hashing]]): a hash of the encoded audio payload with tags excluded, designed to be portable and shareable across machines.

Switch per-track memory to the same content-identity hash, so a track has one identity across the whole app — the hash its playlist entry already references — and its analysis/edits become portable and shareable the way playlists are.

The cost is a re-keying migration: existing entries are keyed the old way and their new key can't be derived without re-hashing each file, so this needs its own handling (re-key lazily on load, or a one-time pass).

Depends on [[track-data-storage]] having relocated per-track memory to its own file first.


## Approach

### Key on content identity

Replace the decoded-PCM hash with the file's content identity (`resilient_playlists::content_hash` of the raw bytes — the same identity playlists and the tag editor already use) as the track database key. It's computed off-thread from the file path at the single load site (`build_deck`'s background pass), so a track carries one identity across the whole app.

No in-app migration. The app carries no code to reconcile old keys — Deck isn't in use by anyone else, so the only data to preserve is the local track database, handled once by an external converter rather than by runtime machinery.

### Content identity is mandatory; failure is a surfaced fault

No PCM fallback. Content identity is the app-wide identity — a track that can't be hashed can't appear in a playlist either, so it is already unsupported; propping it up in the track database with a masquerading key (indistinguishable from a real identity, since both are hex Blake3) would hide that. When identity can't be computed the track still loads and plays, but persists nothing, warns in its deck's notification row, and an incident is recorded to disk. `hash_mono` thereby leaves the runtime entirely (only the converter still uses it), restoring [[remove-rekey-converter]]'s premise.

### Harmonised error reports

Both fault kinds — the tag-edit identity-mismatch and the load-time identity-unhashable — write to one `error_reports/` directory under the state dir, each entry named `YYYY-MM-DD_HHMMSS-<kind>-<label>` so a plain listing sorts chronologically. Mismatches stay folders (original + edited + details); unhashable reports are single text files. A shared `error_reports` module owns the location and naming.

### One-off converter rewrites the existing file

A throwaway converter re-keys the existing `track-data.json` from decoded-PCM hashes to content identities. It scans the browser workspace (the persisted library root) for audio files; for each it decodes to recompute the PCM hash, matches that against the old entries, and writes the matched entry out under the file's content identity — carrying the stored BPM, cue, and gain across so nothing is re-detected. Entries whose files aren't found under the workspace are dropped.

### Converter is a temporary Deck subcommand

It lives as a hidden `deck` subcommand rather than a separate binary, so it reuses the existing decode path and hashing (Deck has no library crate to share with a standalone target). Run once, then removed in a follow-up change — the same retire-after-use pattern as [[remove-cache-migration]].


## Plan

- [x] Key the track database on content identity at the load site, computed from the file path off-thread in place of the decoded-PCM hash
- [x] Add a hidden `rekey-track-data` subcommand that scans the workspace, maps each file's PCM hash to its content identity, and rewrites `track-data.json` under the new keys
- [x] Add an `error_reports` module: single state-dir directory plus dated, type-tagged report naming
- [x] Make load-site identity mandatory (drop the `hash_mono` fallback); signal failure from the load thread
- [x] On identity failure: skip persistence, warn in the deck's notification row, and write an `identity-unhashable` report
- [x] Move the tag-edit identity-mismatch report into the `error_reports` scheme
- [x] Decouple the "analysing" spinner from `analysis_hash` so an unhashable track settles instead of spinning


## Log

- The converter is a hidden `--rekey-track-data` flag, not a clap subcommand, so it coexists with the existing positional path argument.
- Verified end-to-end against `corpus/clean.flac`: a real PCM-keyed entry re-keyed to its content identity with all fields (bpm/offset/cue/gain) preserved; an unmatched entry dropped; summary counts correct.
- The PCM fallback was dropped after review: content identity is mandatory, `hash_mono` leaves the runtime (only the converter uses it), so [[remove-rekey-converter]]'s original "hash_mono becomes dead" premise holds again.
- Unhashable tracks are signalled with an empty-string hash on the existing `(String, f32, i64, bool)` bpm channel, avoiding a channel-type change across `TempoState`, `background_rx`, and the redetect path.
- `analysis_settled` was added to `TempoState` to drive the spinner (with the existing `redetecting` flag), decoupling it from `analysis_hash` — which is now `None` for a settled-but-unhashable track.
- Harmonised reports verified via the incident demo: tag-edit mismatch now lands at `error_reports/YYYY-MM-DD_HHMMSS-identity-mismatch-<label>/`.


## Conclusion

The change grew past the original key-swap: review of the load-time failure case turned it into making content identity mandatory and surfacing an unhashable track as a first-class fault (deck warning + on-disk report), plus harmonising all fault reports under one dated `error_reports/` directory. Shipped as v0.11.45. `hash_mono` now has a single caller (the converter), so [[remove-rekey-converter]] will remove it too.

Documentation impact — map catch-up:

- **Track Database** node: key is now content identity, not the decoded-PCM hash.
- **Metadata Editor** node: identity-mismatch reports now live under `error_reports/`, not `identity-mismatches/`.
- Consider whether error reports warrant their own small node.
