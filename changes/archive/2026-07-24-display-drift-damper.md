# Display Drift Damper

**Mode:** Formal

## Intent

The waveform display drifts visibly out of sync with the audio over long playback. Two decks started at different times accumulate different amounts of drift relative to audio, so they look out of sync with each other even when they sound in sync. Make the drift damper actually effective in steady state, so the display tracks audio continuously rather than accumulating crystal-skew error until the 0.3s snap fires.


## Approach

Root cause: `src/main.rs:2097–2099` recomputes `smooth_display_samp` from `smooth_ref` (anchor + elapsed) every frame, overwriting any damping. The damper at `:2149` runs but its effect is wiped on the next frame. `smooth_ref` only advances on nudge events or the 0.3s snap (which at typical 10–50 ppm skew takes hours to fire), so the display follows the wall clock unbounded.

### Drop `smooth_ref` entirely; integrate per-frame

`service_deck_frame` already receives `elapsed` (the frame interval). Replace the anchor-based integration at `:2097–2099` with `smooth_display_samp += elapsed * rate * speed`. The damper at `:2149` then accumulates frame-to-frame because no one overwrites the running value. Steady-state offset under continuous drift becomes `rate × ε / (k × fps)` ≈ 7 samples (0.17 ms) at 20 ppm with k = 0.002, 60 fps — sub-perceptible.

Per-frame integration of *measured* elapsed telescopes to exact wall time (each `elapsed` is `frame_start − previous frame_start`), so it is mathematically equivalent to the anchor projection — `thread::sleep` jitter cannot compound. The historical jitter problem arose from advancing by *expected* frame duration, which this does not do.

### Integrate with uncapped elapsed

`elapsed` is capped at 4 columns of scroll time (`main.rs:445–448`). That cap can sit *below* the frame duration: at zoom 1 s on a wide terminal the cap is ~20 ms, while `target_fps` 15–30 gives 33–67 ms frames — per-frame integration would then systematically lose time every frame and snap-cycle continuously. The playing-path integration therefore uses the uncapped frame interval; the capped `elapsed` is kept only for the paused-warp scrub path, where the cap's original purpose (bounding scrub jumps after a stall) applies. Both values are computed where `elapsed` is today and passed to `service_deck_frame`.

### Cascading deletions

`smooth_ref` becomes unused. Remove:

- The field on `DisplayState` and its initializer (`src/deck/mod.rs:83`, `:240`).
- Jump-nudge anchor bumps (`src/main.rs:1244`, `:1270`) and the anchor-rationale comment above the first (`:1240–1243`) — the next frame's `+= elapsed × rate × speed` picks up from the bumped `smooth_display_samp` automatically.
- Warp-nudge anchor resets (`:1253`, `:1279`) and the release-path reset (`:1292`) — speed change is reflected via `elapsed × rate × new_speed` next frame. (Speed changes now quantise to frame boundaries; worst-case error is one frame × 10 % ≈ a few ms of audio, absorbed by the damper.)
- Snap-path re-anchor (`:2142`).
- The stale anchor-rationale comment at `:2094–2096`.


## Plan

- [x] Compute an uncapped frame interval alongside the capped `elapsed` at `src/main.rs:445–448`; pass both into `service_deck_frame`.
- [x] Replace anchor-based integration at `src/main.rs:2097–2099` with `smooth_display_samp += elapsed_uncapped * rate * speed`; paused-warp scrub keeps the capped value.
- [x] Delete all `smooth_ref` usage sites: jump-nudge bumps (`:1244`, `:1270`) with the comment at `:1240–1243`, warp-nudge resets (`:1253`, `:1279`, `:1292`), snap-path re-anchor (`:2142`).
- [x] Remove `smooth_ref` from `DisplayState` and its initializer (`src/deck/mod.rs:83`, `:240`).
- [x] Rewrite the comments at `src/main.rs:2094–2096` and `:2143–2148` to describe the per-frame integrate + damp behaviour.
- [x] Bump `Cargo.toml` patch (0.11.1 → 0.11.2).


## Conclusion

Completed as planned; no deviations, empty Log. Tested at v0.11.2. Documentation impact: map catch-up pending on two nodes — *Sliding Viewport* describes the deleted anchor mechanism, and *Partial Drift Correction* states the correction does not persist across frames, which is now the opposite of the implementation.
