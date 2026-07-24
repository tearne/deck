# Waveform Render Efficiency

**Mode:** Formal

## Intent

The scrolling waveform is the most critical part of the application; anything that introduces stutter or incorrect position is highly problematic, especially on the slow hardware the player targets. A rendering audit (2026-07-24) found five drains on smoothness, none of which require architectural change:

1. **Synchronous cache saves on key-repeat paths.** Holding a base-BPM or gain key serialises and writes the cache file on every repeat (~30 Hz) on the UI thread, mid-playback — the most likely real stutter source.

2. **Any change recomputes all three wide buffers.** One deck drifting past its recompute threshold rebuilds all three buffers; a base-BPM ramp rebuilds all three at up to 125 Hz for the duration of the hold, competing with the UI thread for cores.

3. **Per-column colours defeat span merging.** Nearly every detail-waveform column emits its own colour escape sequence — roughly 1.5–2 MB/s of escape-heavy terminal output while scrolling, likely the dominant end-to-end cost on slow hardware. Quantising the spectral colour would multiply run lengths at no visible cost.

4. **Overview rebuilt every frame.** The full-track waveform is recomputed and re-spanned per deck per frame, though its output only changes when the playhead crosses a column or a flash toggles.

5. **Sub-threshold seeks crawl.** With the drift damper fixed (Display Drift Damper change), a playing seek smaller than the 0.3 s snap threshold — e.g. a 1-beat jump above 200 BPM — converges over ~4–8 s, a visible crawl. Steady-state drift is now sub-sample, so the snap threshold can drop substantially, or playing seeks can bump the display position directly.

Goal: eliminate these so scrolling is as close to stutter-free as the terminal allows, and reduce the chance of future regression in the display-position path.

### Asides

Small defects noted in the same audit, candidates to ride along:

- Paused-warp scrub for deck 3 reads deck 2's `samples_per_col` (`src/main.rs`, `service_deck_frame` scrub-granularity lookup).
- `BrailleBuffer` doc comment says the buffer is 3× screen width; the code (and map) use 5×.
- Cover-art lines are cloned every frame despite the size/brightness cache.
