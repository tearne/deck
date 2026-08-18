# Base BPM Pitch

**Mode:** Formal

## Intent

*(Captured as an aside during 20-per-deck-modes.)*

Adjusting a track's base (native) BPM leaves the playback BPM frozen, so the speed ratio `bpm / base_bpm` shifts and the track audibly changes pitch/speed. Setting the base BPM is metadata correction — declaring what the track's true tempo is — and should never affect playback: when `base_bpm` changes, the playback BPM should scale to hold the current speed ratio exactly.

## Approach

Hold the speed ratio invariant: capture `ratio = bpm / base_bpm` before the step, scale `bpm = new_base × ratio` after (same clamps), and drop the `set_speed` call — the speed is the ratio, which doesn't change. The playback BPM display shifts proportionally; grid re-anchoring and persistence unchanged. Aligns the manual keys with tap's existing behaviour.

## Plan

- [x] Base-BPM ramp preserves the speed ratio; no audio call

## Conclusion

Completed at v0.26.1; patch bump confirmed. No map impact — no node claimed the broken behaviour.
