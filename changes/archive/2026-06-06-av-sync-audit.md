# AV Sync Audit

**Mode:** Explore

## Intent

When playing a track on deck 1 against a loop on deck 3, the audio sounds aligned but the waveforms occasionally look out of sync. Determine whether the loop-wrap path causes the display reference to lead the audio playhead by a fade-length offset (~5 ms), and if so, correct the snap timing so the visual and audible playheads stay aligned. If no offset is present after careful code review, document the analysis and close the change with no code changes — the question gets put to bed either way.


## Approach

Code review confirms a `FADE_SAMPLES`-sample (≈5.8 ms) lead of `output_position` over the audio playhead, established on every loop wrap and `SeekHandle.seek_to`. Mechanism: `output_position` is snapped to the target immediately, but the fade-out continues emitting samples from the old position for `FADE_SAMPLES` ticks while `PitchSource` keeps incrementing `output_position` per emitted sample. The drift damper follows `output_position`, so the display inherits the lead. `latency_correction` is a uniform offset and cannot absorb a fade-only one.

### Move the snap to fade-out completion

The `output_position.store(target)` calls in `TrackingSource` (line 220) and `SeekHandle.seek_to` (line 511) move into the fade-out-complete branch (around line 187), beside the existing `position.store(target)`. The display lingers ≈5.8 ms at the pre-wrap position during fade-out — honest, that's what's being heard. After fade-in, position and output_position are aligned.

`seek_direct` and `set_position` are unaffected — they already snap `position` and `output_position` together with no fade, so the lead never arises.


## Plan

### Topics

- Relocate the `output_position` snap from the immediate path (loop wrap, `seek_to`) into the fade-out-complete branch, beside the existing `position.store(target)`.
- Bump `Cargo.toml` patch version for the test build.

### Done when

In `TrackingSource`, the loop wrap and the fade-out-complete branch are the only writers of `output_position` after a wrap/seek, and they leave `output_position == position` at the moment fade-in starts. Verified by a temporary debug log of `output_position - position` printed at fade-in start, confirming `0` across multiple loop wraps on deck 3.


## Conclusion

Loop-wrap snap-timing fix shipped at 0.9.39: the `FADE_SAMPLES`-sample lead of `output_position` over the playhead is structurally eliminated (see Log). No documentation impact.

User testing surfaced a separate AV-sync issue — long-playback drift between non-looping decks, caused by the damper being overwritten each frame by the anchor-based integration. Unrelated to wrap timing; split into a new change.


## Log

- Verification path revised: a runtime debug log in the audio hot path is intrusive and requires the user to read stderr while running the TUI. The fix is provable from the source instead — output_position is now only written in `seek_direct` / `set_position` (no fade, lockstep with `position`) and in the fade-out-complete branch (lockstep with `position`). During fade-in and steady-state playback, `PitchSource` increments `output_position` by one per emitted sample and `TrackingSource` increments `position` by one per `next()`, so they advance together. The persistent `FADE_SAMPLES` lead is structurally eliminated.
- Aside (not addressed here): `PitchSource` pulls inputs in 512-sample chunks when pitch ≠ 0, so `position` briefly leads `output_position` by up to ~11.6 ms during a chunk fill. This is the read-batching jitter the drift damper already exists to smooth (per the comment at `src/main.rs:2106`), unrelated to the wrap path.
- After shipping 0.9.39, user reports residual AV desync between decks 1 and 2 (no looping) that drifts over time during long playback. This is a separate cause from the wrap-snap offset this change addressed: the drift damper at `src/main.rs:2133` is ineffective because the integration formula at `:2082` overwrites `smooth_display_samp` from `smooth_ref` each frame. The damper's per-frame correction is wiped before it accumulates, so clock-skew drift between wall clock and audio device clock grows linearly until the 0.3s snap fires (hours, at typical 10–50 ppm skew). Splitting this into a new change.
