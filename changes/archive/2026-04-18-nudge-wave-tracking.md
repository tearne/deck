## Intent

When nudging during playback — in either jump or warp sub-mode — the detail waveform display no longer tracks the audio smoothly. Instead of the waveform scrolling in lockstep with what the DJ hears, the display jumps, giving a misleading view of the current playback position.

The fix should restore the shared frame of reference between the audio and the detail waveform: whatever position the DJ hears is the position the waveform shows.

## Approach

Run this change as an **investigation**, not an upfront plan. The builder explores the code and behaviour ad hoc, gathering evidence about where audio and display diverge under each nudge sub-mode. No fix is designed or applied in this pass.

On completion, the builder writes a Feedback section summarising the evidence and what the planner should reconsider. The change then returns to plan mode so a proper fix can be planned from the evidence.

- Scope: diagnosis only. Any code changes during investigation (e.g. temporary instrumentation) must be reverted before handing back.
- Review cadence: at the end, when Feedback is written.

## Plan

- [x] INVESTIGATE nudge-mode display tracking — reproduce in jump and warp sub-modes, gather evidence on where audio and display diverge
- [x] WRITE FEEDBACK — summarise findings and flag what the planner should reconsider
- [x] APPLY HYPOTHESIS FIX — invalidate `smooth_ref` on nudge press/release so the time-anchor re-establishes at the new discontinuity
- [x] BUMP VERSION — patch bump for testable build (0.9.20 → 0.9.21)
- [x] RELEASE BUILD — user tests the candidate fix; evidence feeds a further Feedback entry
- [x] ADD capture state (key binding, active-window timer, per-frame row buffer)
- [x] ADD rebuild_count atomic, bumped on each background rebuild
- [x] WRITE CSV on capture end with notification
- [x] BUMP VERSION — 0.9.21 → 0.9.22
- [x] RELEASE BUILD — user captures a hold-nudge session; evidence feeds a further Feedback entry
- [x] EXTEND CSV fields — `nudge_events` (per-deck event counter) and `display_speed` so jump-mode activity and the speed used by the anchor formula are visible
- [x] RELEASE BUILD v0.9.23 — second capture confirms 30 Hz key-repeat and isolates the drift mechanism
- [x] APPLY FIX — in jump-mode handlers, shift `smooth_ref.anchor_sample` by the same bump rather than dropping the anchor
- [x] RELEASE BUILD v0.9.24 — user confirms jump-mode display now tracks audio cleanly

## Feedback

**Status:** partially implemented — jump-mode fix applied at v0.9.24 and confirmed by the user; warp mode not re-verified under the current code. Further plan work required to address warp (if it still misbehaves) and to update the Sliding Viewport node in `map.md`.

**Notes**

The Detail Waveform display is driven by `smooth_display_samp`, which in the current (committed) code is computed each frame from a fixed time anchor `smooth_ref: Option<(Instant, f64)>`:

```
smooth_display_samp = anchor_sample + (now − anchor_time) × sample_rate × speed
```

This anchor is established once on entering the play branch and persists across frames. The background buffer thread reads `smooth_display_samp` (via `display_pos_a/b/c`) and recomputes the wide buffer when drift from the last anchor exceeds 75% of the screen; a drift-correction step pulls `smooth_display_samp` toward the audio's `output_position`, snapping to a half-column if divergence exceeds 0.3 s.

Nudge in the committed code never invalidates `smooth_ref`. That is the root cause of the jump in both sub-modes:

- **Jump sub-mode.** The press handler writes `smooth_display_samp += (target − current) × sample_rate` alongside the audio seek. Because the time anchor is unchanged, the very next frame overwrites `smooth_display_samp` with `anchor_sample + elapsed × sr × speed`, erasing the bump. Audio has moved; display has not. Repeated presses let divergence grow until the 0.3 s drift-snap fires, producing the visible jump.

- **Warp sub-mode.** The press handler changes audio speed (`set_speed(... × 1.1)` or `× 0.9`) without touching the anchor. The next frame still uses the old `(anchor_time, anchor_sample)` pair but with the new speed factor, so the formula evaluates as if the new speed had been in effect since `anchor_time`. The result is an instantaneous jump of `(t_press − anchor_time) × sample_rate × 0.1` in `smooth_display_samp` — larger the longer the anchor has been active before the press. Release has the symmetric effect.

A hypothesis that appears true in both modes: the fix is to force a re-anchor at each discontinuity — invalidate `smooth_ref` on every nudge press and on warp release. The per-frame `get_or_insert_with` then re-anchors at the current `smooth_display_samp` and current `Instant::now()`, and subsequent frames advance correctly at the active speed.

Secondary concern (not the primary cause of the reported jump, but worth flagging): warp's `set_speed` uses `d.tempo.bpm / d.tempo.base_bpm * 0.9` (or `* 1.1`) regardless of vinyl mode. In vinyl mode the player is normally running at `vinyl_speed`, not at `bpm/base_bpm`, so warp in vinyl mode temporarily overrides the user's vinyl-speed setting and, on release, returns to `bpm/base_bpm` rather than `vinyl_speed`. The display's `base_speed` branch already handles vinyl vs beat; audio speed should use the same branching.

**Documentation impact**

- `map.md` — the Sliding Viewport node describes the time-anchored display but does not capture the invariant that the anchor must be invalidated on any audio-side discontinuity. Worth making this explicit when the fix lands.
- No spec change expected.

**Follow-up — hypothesis fix was not sufficient**

v0.9.21 applied `smooth_ref = None` at each of the four nudge discontinuities (backward-jump press, forward-jump press, warp press in either direction, warp release). The user tested and reported "abrupt jumps every ~1s as I hold down the nudge in either direction while playing". So:

- The press-time retroactive-jump that the anchor invalidation addresses is likely gone, but
- A **periodic** ~1 s artefact persists during the hold itself, which the anchor-invalidation fix cannot explain.

Candidate causes not yet ruled out:
- Periodic re-anchor or drift snap during steady-state warp (would need to observe `smooth_ref` resets and the 0.3 s snap to confirm).
- Key-repeat events during the hold are re-firing the press handler (`Press | Repeat`), which re-calls `set_speed` and `smooth_ref = None` on every repeat. On a typical keyboard this is ~30 Hz, not ~1 Hz — but a slow initial-repeat delay, or something in the handler that only perturbs the display on certain repeats, could produce the observed cadence.
- Wide-buffer rebuild at drift ≥ 75 % of screen width. At zoom 4 s and 1.1× warp, rebuild fires about every 2–3 s. Rebuilds should be invisible by construction (same audio content, same col_samp), but this is unverified under warp.
- Vinyl mode secondary issue (warp `set_speed` uses `bpm/base_bpm × 1.1` instead of `vinyl_speed × 1.1`) — may be contributing if testing was done in vinyl mode.

The investigation has exhausted productive code-reading without more evidence. The rational next step is **targeted instrumentation** — e.g. a small overlay or short-window CSV capture that records per frame `smooth_display_samp`, `output_position`, computed `drift`, a flag for `smooth_ref` being reset, and a rebuild counter from the background thread — so a hold-nudge session can be inspected directly. This overlaps meaningfully with the pending `display-diagnostics` change and might be best merged into that rather than bolted onto this one.

**Follow-up — CSV capture isolates the real cause (jump mode)**

A `\`-toggled 10-second capture (v0.9.22, extended in v0.9.23 with `nudge_events` and `display_speed` columns) was added and exercised against a held forward-jump nudge. The first capture showed the symptom but `d.nudge` read 0 throughout — that field only tracks warp-mode state, so jump-mode activity was invisible. The `nudge_events` counter confirmed 30 Hz key-repeat throughout the second capture.

Rate analysis on the second capture:

- Display advance: ~45,400 samples/sec
- Audio `output_position` advance: ~58,000 samples/sec
- Expected display rate at 30 events/sec × 10 ms bump + 1.0× base: ~57,300 samples/sec
- Shortfall: ~12,000 samples/sec on the display side

Tracing frame-by-frame: on every nudge event the handler sets `smooth_ref = None`. The next `service_deck_frame` call re-anchors at `(Instant::now(), current_smooth_display_samp)`. Any real time elapsed between the event firing and the next service call — roughly 9 ms on a 10 ms frame — is *not* credited to natural advance because the anchor clock has just been reset. At 30 events/sec this loses ~270 ms per second, or about 27 % of the natural advance rate. `44,100 × 0.73 + 13,230 ≈ 45,400` — matches observation exactly.

The drift accumulates until it crosses the 0.3 s snap threshold, which then fires and yanks `smooth_display_samp` forward by ~13,000 samples. Period: one second. That is the visible jump.

**Fix applied (v0.9.24)**

Both jump-mode handlers now shift the anchor sample by the same bump rather than dropping the anchor:

```rust
let bump = (target - current) * d.audio.sample_rate as f64;
d.audio.seek_handle.set_position(target);
d.display.smooth_display_samp += bump;
if let Some((_, ref mut anchor)) = d.display.smooth_ref {
    *anchor += bump;
}
```

The anchor clock keeps running across the event, so no wall-time is lost; the anchor sample absorbs the discontinuity so the next frame's `anchor_sample + elapsed × sr × speed` preserves the jump. User tested v0.9.24 and confirmed jump-mode tracking is now smooth.

**Warp mode — not verified with this fix**

The v0.9.21 `smooth_ref = None` reset is still in place for the four warp paths (press ±, release). Warp is different: it changes `speed`, not position, so the anchor *must* be re-established at the speed transition — simply shifting `anchor_sample` would not suffice because the formula's slope changes. The user reports jump-mode is the mode they exercise; warp has not been re-tested against the original "abrupt jumps every ~1 s" symptom under the current code. If warp still exhibits the artefact, the mechanism is likely different (candidates from the earlier Feedback remain: batched `output_position` batch-arrival jitter, vinyl-mode speed-base mismatch in `set_speed`).

**Documentation impact**

- `map.md` Sliding Viewport node: should capture the invariant that the anchor sample must be shifted to track audio-side discontinuities, and the anchor clock must not be reset except at genuine speed transitions.
- No spec change expected.

## Conclusion

Jump-mode tracking fixed at v0.9.24 by shifting the anchor sample on each nudge press rather than dropping the anchor. Warp-mode remediation and the `map.md` Sliding Viewport update are deferred to a follow-up change (same mechanism is assumed to apply). The diagnostic instrumentation added during investigation (`\` capture key, per-frame CSV writer, `rebuild_count` atomic, `nudge_event_count` array, `DiagRow`/`DiagCapture` types) is left in the working tree and will be tracked for removal under a separate sibling change rather than reverted now — the Approach's "diagnosis only" clause is superseded by this decision.
