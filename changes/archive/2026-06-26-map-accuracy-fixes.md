# Map Accuracy Fixes

**Mode:** Wander

## Intent

The consistency check against the source code found three inaccuracies in the map:

1. **Beat Jump sizes** — map says four sizes (1, 4, 16, 64 beats). Code has seven: 1, 4, 16, 32, 64, 128, 256 beats. The config uses `bt` (beat) and `b` (bar) suffixes: 1bt, 4bt, 4b, 8b, 16b, 32b, 64b.

2. **Tap BPM minimum** — map says "After 8 taps". Code computes from 2 taps onward (`if n < 2 { return }`), continuously updating as more taps arrive. The SPEC also says 8, which appears to be stale.

3. **Audio Pipeline stage count** — map lists three separate stages (Filter → Level & Gain → Pitch Shift). In the code, filter, level, gain, and PFL are all handled inside `FilterSource` as a single processing stage, followed by `PitchSource`. The map's numbered list implies three distinct stages but there are really two.


## Conclusion

One genuine inaccuracy fixed — Beat Jump sizes corrected from "four sizes (1, 4, 16, 64 beats)" to "seven sizes — 1 beat, 1 bar, 4 bars, 8 bars, 16 bars, 32 bars, 64 bars", using bar notation consistent with the in-app overlay. Items 2 and 3 from the intent were false positives: "After 8 taps" matches the caller threshold, and the Audio Pipeline node describes logical steps not implementation structs. A new aside `rename-4bt-jump.md` captures the follow-on config action rename.
