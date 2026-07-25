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


## Approach

### Debounced cache persistence

All mid-session `cache.save()` calls are removed; mutation marks the cache dirty with a timestamp, and the main loop flushes once the cache has been dirty and untouched for ~1 s. Quit paths keep an unconditional flush. This removes synchronous JSON serialisation and file writes from key-repeat paths; a ≤1 s crash-loss window is acceptable for BPM/gain trims.

### Per-slot wide-buffer recompute

The single `must_recompute` flag splits per slot: cols/rows/zoom changes rebuild all three; col_samp, drift, load generation, bpm, offset, cue, gain, and loop changes rebuild only their own slot. Steady-state drift rebuilds drop to a third of today's cost; single-deck events stop touching the other decks.

### Settle debounce for parameter-driven rebuilds

Parameter-driven rebuilds (bpm, offset, gain, speed ratio) wait until the value has been stable ~50 ms; drift, resize, zoom, and load rebuild immediately. This ends the rebuild storm during a held base-BPM ramp (value changes at ~30 Hz, thread polls at 125 Hz). Trade-off: buffer scale and ticks freeze during the hold, catching up ~50 ms after the last repeat.

### Spectral colour quantisation

Bass ratio is quantised to 32 levels at colour-lookup time via a per-palette lookup table; buffer data keeps full precision. Today every column gets a unique RGB, so run-length merging never merges and each column emits its own SGR sequence; box-smoothed neighbours quantised to shared levels collapse into runs, cutting span allocations and terminal output several-fold.

### Overview line cache

Rendered overview lines are cached per deck (following the `cover_art_cache` pattern), keyed on the derived inputs: size, playhead column, cue column, flash states, analysing, gain, palette, bpm/offset. The output changes at most a few times per second but is rebuilt at frame rate today. The rebuild also replaces the per-cell `bar_cols.contains()` scan with a column mask.

### Playing snap threshold 0.3 s → 0.1 s

With the damper fixed, steady-state drift is sub-millisecond and audio batch noise is under ~25 ms, so 0.1 s stays far above noise while catching seeks the old threshold missed: a 1-beat jump above 200 BPM (0.25 s at 240 BPM) currently falls below 0.3 s and crawls in over seconds. Preferred over plumbing seek deltas into the display position, which would duplicate seek semantics across call sites.

### Asides resolution

Scrub lookup gains the missing slot-2 arm; the `BrailleBuffer` width comment corrects to 5×; the cover-art cache stores a `Paragraph` rendered by reference (ratatui 0.30 renders `&Paragraph`), eliminating the per-frame clone.


## Plan

- [x] Add dirty-flag + last-mutation timestamp to `Cache`; mutators mark dirty; idle flush (≥1 s untouched) called once per frame; quit paths flush unconditionally.
- [x] Replace all mid-session `cache.save()` calls with the dirty-marking path.
- [x] Split `must_recompute` into per-slot triggers (cols/rows/zoom rebuild all; col_samp, drift, load gen, bpm, offset, cue, gain, loop rebuild their slot only).
- [x] Add 50 ms settle debounce to parameter-driven rebuild triggers; drift, resize, zoom, and load stay immediate.
- [x] Quantise bass ratio to 32 levels behind a per-palette colour LUT in detail and overview rendering.
- [x] Cache overview lines per deck keyed on derived inputs; replace `bar_cols.contains()` with a column mask.
- [x] Lower the playing drift-snap threshold to 0.1 s and update its comment.
- [x] Fix slot-2 paused-warp scrub to read its own buffer's `samples_per_col`.
- [x] Correct the `BrailleBuffer` width comment to 5×.
- [x] Store the cover-art cache as a `Paragraph` and render it by reference.
- [x] Bump `Cargo.toml` patch (0.11.2 → 0.11.3).


## Log

- The background renderer thread body was restructured from a/b/c-suffixed locals to per-slot arrays; the per-slot rebuild decision made the triplicated form untenable.
- Deck 3's loop bounds ride the settle-debounced parameter path alongside bpm/offset/gain/speed; loop activation therefore reaches the buffer ~50 ms after the tap session ends (the audio-side loop atomics are unaffected).
- `gen` is a reserved keyword in edition 2024; the load-generation local is named `load_gen`.
- A reported cue-set regression during hand-back testing was traced (via a pty harness driving the real binary against a null ALSA device) to a stale local `config.toml`, not this change: commit cb035b8 rebound cue to `B`/`G`, and a pre-overhaul local config overlays `B → base_bpm_increase` and `G → pfl_on_off` over the new seeds. `resolve_or_create` never refreshes an existing file and `dev-build-run.py` always passes `--local-config`.


## Conclusion

Completed as planned; patch bump to 0.11.3 confirmed. Hand-back testing spun off two follow-on proposals: `dev-build-run-config-refresh` (from the stale-config trap) and `render-stutter-exploration` (residual stutter on target hardware). Map catch-up pending on four nodes: Cache (persistence now debounced ~1 s, flush on quit), Drift Snap (playing threshold 0.3 s → 0.1 s), Wide Buffer (per-slot recompute triggers, 50 ms settle debounce on parameter-driven rebuilds), Spectral Colour (32-level quantisation at colour lookup).
