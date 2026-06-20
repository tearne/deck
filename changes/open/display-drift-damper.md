# Display Drift Damper

**Mode:** Formal

## Intent

The waveform display drifts visibly out of sync with the audio over long playback. Two decks started at different times accumulate different amounts of drift relative to audio, so they look out of sync with each other even when they sound in sync. Make the drift damper actually effective in steady state, so the display tracks audio continuously rather than accumulating crystal-skew error until the 0.3s snap fires.


## Approach

Root cause: `src/main.rs:2082` recomputes `smooth_display_samp` from `smooth_ref` (anchor + elapsed) every frame, overwriting any damping. The damper at `:2133` runs but its effect is wiped on the next frame. `smooth_ref` only advances on nudge events or the 0.3s snap (which at typical 10–50 ppm skew takes hours to fire), so the display follows the wall clock unbounded.

### Drop `smooth_ref` entirely; integrate per-frame

`service_deck_frame` already receives `elapsed` (the frame interval) at `:468`. Replace the anchor-based integration at `:2081–2083` with `smooth_display_samp += elapsed * rate * speed`. The damper at `:2133` then accumulates frame-to-frame because no one overwrites the running value. Steady-state offset under continuous drift becomes `rate × ε / (k × fps)` ≈ 7 samples (0.17 ms) at 20 ppm with k = 0.002, 60 fps — sub-perceptible.

Risk: per-frame accumulation exposes `thread::sleep` jitter in `elapsed`, which the anchor approach was originally chosen to avoid (per the comment at `:2078–2080`). With k = 0.002 the damper barely reacts within a frame, so jitter amplification should be negligible — but if it shows up visibly, fallback is a one-line re-anchor of an anchor we'd keep around.

### Cascading deletions

`smooth_ref` becomes unused. Remove:

- The field on `DisplayState` and its initializer (`src/deck/mod.rs:83`, `:240`).
- Jump-nudge anchor bumps (`src/main.rs:1228`, `:1254`) — the next frame's `+= elapsed × rate × speed` picks up from the bumped `smooth_display_samp` automatically.
- Warp-nudge anchor resets (`:1237`, `:1263`, `:1276`) — speed change is reflected via `elapsed × rate × new_speed` next frame.
- Snap-path re-anchor (`:2126`).
- The stale comment at `:2128–2132`.


## Plan

- [ ] Replace anchor-based integration at `src/main.rs:2081–2083` with `smooth_display_samp += elapsed * rate * speed`.
- [ ] Delete all `smooth_ref` usage sites: jump-nudge bumps (`:1228`, `:1254`), warp-nudge resets (`:1237`, `:1263`, `:1276`), snap-path re-anchor (`:2126`).
- [ ] Remove `smooth_ref` from `DisplayState` and its initializer (`src/deck/mod.rs:83`, `:240`).
- [ ] Rewrite the comment at `src/main.rs:2128–2132` to describe the per-frame integrate + damp behaviour.
- [ ] Bump `Cargo.toml` patch.
