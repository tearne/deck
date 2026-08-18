# BPM Position

**Mode:** Wander

## Intent

*(Captured as an aside during 15-browser-deck-select.)*

The BPM readout has no uniform position — it sits in the right-anchored readout line, so it shifts left or right as the offset text beside it changes width. Move it to a fixed spot at the deck's top-left, potentially replacing the play/pause icon, which adds little.

## Conclusion

Completed at v0.26.2 in one iteration. The tempo group (BPM/percentage, pitch, metronome note, and the analysing spinner) moved from the readout's head to the title corner, displacing the play icon; the grid offset moved to the readout's head behind the mode tag. The fix works because everything left of the tempo group is constant-width — just the deck number — giving BPM a true fixed column, which the right-anchored readout could never provide. Play state is carried by waveform motion. Overview Waveform's corners sentence updated alongside.
