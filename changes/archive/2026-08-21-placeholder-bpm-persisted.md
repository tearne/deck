# Placeholder BPM Persisted

**Mode:** Formal

## Intent

A new track with no BPM set opens in Playback mode, as intended. On reopening it later, it comes up in Beat mode with a BPM of 120 — the placeholder — though the operator never set one. Somewhere between load and the next open, the placeholder is being saved as if it were a real tempo (and/or Beat mode is being remembered for a track that never had a grid). A track without a tempo the operator chose should keep opening in Playback mode with no BPM.

Observed 2026-08-21 while testing ghost playheads (0.29.14); the track had been opened and played, but no tap, detection, or manual adjust was used.

## Approach

### What actually happens

A fresh track opens in **Beat** mode (the default when the database has no entry), but with the tempo marked unconfirmed — and an unconfirmed Beat deck deliberately looks like Playback: percentage readout, no grid. The track database record, however, stores only the number: the 120 placeholder is written on the very first load (and on every gain, cue, or quit thereafter), and on the next open the mere presence of a record is read as "confirmed". The placeholder has been promoted to a tempo by round-tripping.

### The record stores a grid or none

The database record's flat `bpm` / `offset_ms` become one nullable `grid` object holding both, named for the map's concept (the pair *is* the beat grid). `null` means never confirmed — absence is the encoding, so no flag can disagree with the number. On load a record without a grid restores cue, gain, anchor, and mode but leaves the tempo at the placeholder and unconfirmed. The write-only `offset_established` flag is dropped: offset has no meaning without a tempo and rides inside `grid`.

### Anchor stays top-level

An anchor can be pinned before the tempo is confirmed and the offset is derived from it afterwards, so it is not part of the nullable grid.

### Legacy records migrate on read

Flat `bpm` / `offset_ms` fields are read as a confirmed grid and rewritten in the new shape at the next save, following the loader's existing fallback for old flat files. Placeholder records written before this fix therefore read as confirmed 120 — no heuristic tries to guess otherwise.

### Nothing else changes

Writing a record on first load stays: it is what carries mode, gain, and cue for a track the operator never tapped. The Beat-by-default open also stays — the Intent's "opened in Playback" was this unconfirmed-Beat look, and the fix makes reopening look the same. Manual base-BPM adjust (`V`/`F`) keeps confirming the tempo: it is the only way to enter a known BPM by hand, and nobody steps the native BPM by 0.01 by accident.

## Plan

- [x] Replace `bpm` / `offset_ms` / `offset_established` on the database record with `grid: Option<Grid { bpm, offset_ms }>`
- [x] Read legacy flat records as a confirmed grid, with a test covering the old shape and a `null` grid round-trip
- [x] Write the record from the deck's live state: `grid` is `Some` only when the tempo is confirmed
- [x] Load: a record without a grid takes the placeholder and leaves the tempo unconfirmed; cue, gain, anchor, and mode restore as before
- [x] Remove the deck's `offset_established` field and its writers
- [x] Hand back: open a never-tapped track, quit, reopen — still unconfirmed; a tapped track still reopens confirmed

## Log

- Record reads through a raw shape accepting both `grid` and legacy flat `bpm`/`offset_ms`; a legacy `offset_established` field is ignored on read and no longer written. Files rewrite in the new shape at their next save.
- 0.29.18 for hand-back.

## Conclusion

Completed; patch bump confirmed (0.29.18). Records written before this fix for played-but-never-tapped tracks still carry a confirmed 120 — tap or delete those individually. Map catch-up pending: Track Database (record shape: nullable grid, no offset-established flag) and Beat Grid (cache lookup applies only a confirmed grid).
