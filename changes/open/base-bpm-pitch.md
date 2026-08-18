# Base BPM Pitch

**Mode:** Formal

## Intent

*(Captured as an aside during 20-per-deck-modes.)*

Adjusting a track's base (native) BPM leaves the playback BPM frozen, so the speed ratio `bpm / base_bpm` shifts and the track audibly changes pitch/speed. Setting the base BPM is metadata correction — declaring what the track's true tempo is — and should never affect playback: when `base_bpm` changes, the playback BPM should scale to hold the current speed ratio exactly.
