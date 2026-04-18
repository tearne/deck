# Anchor Never Drop

## Intent

The jump-nudge fix established that the display's time-anchor must carry every position or speed change without being forfeited — dropping the anchor loses wall-time between the event and the next frame, compounding into a ~1 s periodic drift-snap. Warp press and release were not brought under that principle and are presumed to still exhibit the symptom.

The broader idea is that the anchor always has a meaningful value: it is initialised when a deck is constructed and updated whenever the displayed trajectory needs to change (drift snap, position jump, speed change). There is no "not yet set" state. Aligning the code with this model closes the warp gap and removes the residual "anchor absent" branches that the jump fix left in place.

## Approach

**Warp press/release update pattern.** On a warp transition the audio speed changes but position is continuous. The anchor must be refreshed so the next frame computes from the new speed only: set the anchor's time to `Instant::now()` and its sample to the current `smooth_display_samp`. Replaces the three `smooth_ref = None` assignments in the warp handlers.

**Option removal.** Change `smooth_ref: Option<(Instant, f64)>` to `smooth_ref: (Instant, f64)` in `deck/mod.rs`. Initialise at deck construction with `(Instant::now(), 0.0)` — overwritten by the first meaningful update (load, drift snap, or first frame) before any user-visible drift can accumulate.

**Jump-handler simplification.** The three jump handlers currently guard the anchor update with `if let Some((_, ref mut anchor)) = d.display.smooth_ref`. Collapse to direct tuple-field access: `d.display.smooth_ref.1 += bump`.

**Drift-snap and first-frame sites.** `get_or_insert_with` at the frame-service site becomes a plain read (the anchor is always seeded). The drift-snap re-anchor drops its `Some(...)` wrapper.

**Diagnostics.** The `smooth_ref_present` field and CSV column lose meaning with Option removed; drop them as part of this change rather than leave a misleading `always true` value until Change B removes the instrumentation.

**Version bump.** `Cargo.toml` patch bump `0.9.24 → 0.9.25`.

**Review cadence.** End-of-build review of the full diff.

## Plan

- [x] UPDATE IMPL: change `smooth_ref` field type in `deck/mod.rs` from `Option<(Instant, f64)>` to `(Instant, f64)`; initialise constructor with `(Instant::now(), 0.0)`.
- [x] UPDATE IMPL: replace `get_or_insert_with` at the frame-service site in `main.rs` with direct read of `smooth_ref`.
- [x] UPDATE IMPL: drop the `Some(...)` wrapper at the drift-snap re-anchor site in `main.rs`.
- [x] UPDATE IMPL: collapse `if let Some((_, ref mut anchor)) = d.display.smooth_ref { *anchor += bump; }` to `d.display.smooth_ref.1 += bump;` in the two jump handlers.
- [x] UPDATE IMPL: replace the three warp-handler `smooth_ref = None` assignments with `smooth_ref = (Instant::now(), d.display.smooth_display_samp);`.
- [x] UPDATE IMPL: remove the `smooth_ref_present` field from `DiagRow` and its CSV column header and write-format lines.
- [x] UPDATE VERSION: bump `Cargo.toml` patch `0.9.24 → 0.9.25`.
- [x] VERIFY: manual test — warp-nudge during playback tracks smoothly; no periodic ~1 s drift-snap; jump-nudge behaviour unchanged from v0.9.24.
- [x] REVIEW: end-of-build walk through the full diff with the user.

## Conclusion

Completed. Minor cleanup beyond the plan: the stale warp-handler comment describing the old "drop the anchor" behaviour was removed, and the jump-handler comment was shortened now that dropping is no longer a codebase concern.
