# Display Drift Damper

**Mode:** Formal

## Intent

The waveform display drifts visibly out of sync with the audio over long playback. Two decks started at different times accumulate different amounts of drift relative to audio, so they look out of sync with each other even when they sound in sync. Make the drift damper actually effective in steady state, so the display tracks audio continuously rather than accumulating crystal-skew error until the 0.3s snap fires.


## Approach

Root cause: `src/main.rs:2082` recomputes `smooth_display_samp` from `smooth_ref` (anchor + elapsed) every frame, overwriting any damping. The damper at `:2133` runs but its effect is wiped on the next frame. `smooth_ref` only advances on nudge events or the 0.3s snap (which at typical 10–50 ppm skew takes hours to fire), so the display follows the wall clock unbounded.

### Re-anchor `smooth_ref` after each damp

Right after `smooth_display_samp -= drift * 0.002` at `:2133`, set `smooth_ref = (Instant::now(), smooth_display_samp)`. The next frame's integration starts from a near-zero elapsed window, so the damper's correction persists. Steady-state offset under continuous drift becomes `rate × ε / (k × fps)` ≈ 7 samples (0.17 ms) at 20 ppm with k = 0.002, 60 fps.

### Keep the damper factor at 0.002

With re-anchoring, 0.002 yields a low-pass time constant of ~8 s — long enough to swallow per-sample audio-device step noise (the original concern in the existing comment) and short enough to keep steady-state drift sub-perceptible. No retune needed.

### Update the stale comment

The "elapsed-advance removing steady-state lag" claim at `:2128–2132` predates the realisation that the damper was being wiped. Rewrite to describe the re-anchored low-pass behaviour.


## Unresolved

- Whether to also strip the explicit `smooth_ref` re-anchors in the nudge handlers (`:1237`, `:1263`, `:1276`). With per-frame re-anchoring they're redundant but harmless. Leaning leave-in-place for now to keep this change tightly scoped.
