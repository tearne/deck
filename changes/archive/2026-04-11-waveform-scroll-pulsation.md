# Waveform Scroll Pulsation

## Intent

During playback the detail waveform scrolls with a visible rhythmic pulsation — alternating faster and slower — at roughly a 1-second period. The effect has been present since at least the playback-bugs commit and makes the display feel unsteady.

## Approach

`smooth_display_samp` advances using the nominal `frame_dur` each iteration. `output_position` advances at the real audio rate. Because `thread::sleep` overshoots, `output_position` consistently runs ahead, creating a steady-state lag that the `drift * 0.05` correction then has to fight. The correction factor gives a settling time constant of roughly one second, which amplifies any ~1Hz periodic perturbation in frame timing (CPU frequency scaling, OS scheduling) into visible scroll-speed oscillation.

Options A (elapsed-based advance), C (large_drift threshold 0.1 → 0.3), and D (correction factor 0.05 → 0.002) were applied and shipped at v0.9.16. Option D gave a clear, visible improvement. Option E (f64 precision for display_samp) was applied and live-toggled — no visible difference; removed.

Residual jitter persists. The builder's extended experimentation established the following:

- Removing the drift correction block entirely (including `large_drift` snap) produced no visible change — both original post-build candidates are ruled out. The jitter source is not in the correction or snap path.
- The background renderer swaps in a new `Arc<BrailleBuffer>` behind a mutex, but `buf_a`/`buf_b`/`buf_c` are Arc-cloned before each frame (line ~480) and `DeckRenderState.display_pos_samp` derives from `smooth_display_samp`, not the live `output_position` atomic — mid-frame inconsistency is not possible.
- The residual jitter is in the base advance: `smooth_display_samp += elapsed * sample_rate * speed`. `thread::sleep` overshoot creates anti-correlated `elapsed` values across frames — a long frame forces a shorter next frame — causing `smooth` to oscillate around the true position. At `half_col_samp ≈ 330 samples`, frame-to-frame oscillation of similar magnitude is sufficient to toggle `delta_half` between adjacent integers.

**Option F (experiment)**: Replace the accumulated `elapsed`-based advance with an absolute time reference. Track a `(Instant, f64)` anchor — reset on playback start and on seek. Each frame: `smooth_display_samp = anchor_sample + (now - anchor_time).as_secs_f64() * sample_rate * speed`. This eliminates per-frame accumulation jitter entirely. The `large_drift` snap remains as a fallback for seek recovery.

Implemented behind a live toggle key so the builder can switch between modes during playback and observe the difference directly. The builder reports back via Feedback with observations.

Review cadence: at the end.

## Plan

- [x] ADD smooth_reference anchor `(Instant, f64)` to display state; initialize on playback start, reset on seek
- [x] ADD toggle (single key) to switch `smooth_display_samp` advance between elapsed-accumulation and absolute-reference modes; default off (current behaviour)
- [-] TEST live: play a track, toggle between modes, observe whether absolute-reference mode reduces visible jitter
- [-] REVIEW report back in `## Feedback` with delivery status and toggle observations — note whether modes are distinguishable and which is better

## Log

After shipping abs-reference mode, user noted no visible improvement. Since `delta_half` toggling and `samples_per_col` instability are the two remaining candidates, add a `--diag` flag that writes a per-frame CSV (`deck-diag.csv`) logging `frame, deck, display_pos_samp, delta_half, samples_per_col, sub_col, drift` for each active deck. No UI impact when flag is absent.

## Conclusion

Toggle implemented and tested live. No visible difference between modes was observed. On the basis that absolute-reference is theoretically superior (no per-frame accumulation error), the toggle was removed and absolute-reference advance hardcoded as the only path. The `AbsAdvanceToggle` action, its key binding (`\`), and the `[abs]` status bar indicator were all removed. `frame_dur` was also removed from the `service_deck_frame` signature as it became unused. Shipped at v0.9.17.

Diagnostic logging (`--diag`) was added to characterise the residual jitter. Analysis showed `samples_per_col` is perfectly stable and `delta_half` advances cleanly with only expected Bresenham dithering (~1 skip per 7 frames). The position pipeline is not the source of any remaining pulsation; investigation should shift to the renderer. The `--diag` flag was removed before shipping.

`target_fps` config key added to allow frame rate floor override for further experimentation. A follow-on change should expose the frame rate cap as a live, in-UI control so it can be adjusted during playback without editing config.

The diagnostic work suggests a case for a formal display diagnostics feature. A possible design: the user presses a key to trigger a 10-second capture window, after which a CSV is written to disk. The planner should think carefully about what statistics are most useful (candidates from this investigation: `frame`, `deck`, `display_pos_samp`, `delta_half`, `samples_per_col`, `sub_col`, `drift`, and potentially `anchor_sample` and frame wall-clock timestamps), as well as column naming conventions, file naming, and where the output path is communicated to the user. The feature should be designed to be stable and self-describing enough that a capture can be analysed without reference to the source code.
