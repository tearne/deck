# Remove Nudge Diagnostics

## Intent

During the nudge-wave-tracking investigation, diagnostic instrumentation was added to the live binary: a `\` key toggle starts a 10 s per-frame CSV capture covering `smooth_display_samp`, `output_position`, drift, anchor presence, rebuild count, nudge events, and display speed. The rendering thread's `rebuild_count` atomic was added to surface wide-buffer rebuilds. A `nudge_event_count` array tracks jump-mode key activity that `d.nudge` does not.

This instrumentation was load-bearing for the fix and is being left in place for the follow-up warp investigation. Once the warp work is complete and no further live-session captures are needed, this change will remove the instrumentation so the binary is not shipped with an ad-hoc debugging path.

## Approach

**Scope.** Strip everything added for the nudge investigation:

- In `main.rs`: `DiagRow` struct, `DiagCapture` struct, `write_diag_csv` function, `DIAG_CAPTURE_DURATION` constant, the `diag_capture` and `nudge_event_count` state, the per-frame row-collection block, the `\` key handler, and the two `nudge_event_count` increment sites in the jump handlers.
- In `render/mod.rs`: the `rebuild_count` atomic — its only consumer was the diagnostic capture, so it comes out alongside.
- Any imports left unused (`io`, `PathBuf`, etc.) — the compiler will flag these during removal.

**Version bump.** Patch bump `Cargo.toml` `0.9.25 → 0.9.26` for the testable build.

**Review cadence.** End-of-build review of the full diff — the change is mechanical and the compiler catches unused imports.

## Plan

- [x] REMOVE IMPL: `\` key handler in `main.rs`.
- [x] REMOVE IMPL: the per-frame row-collection block that builds `DiagRow`s and triggers the CSV write.
- [x] REMOVE IMPL: `diag_capture` and `nudge_event_count` state declarations in the main loop, and the two `nudge_event_count` increment sites in the jump handlers.
- [x] REMOVE IMPL: `DiagRow`, `DiagCapture`, `write_diag_csv`, and `DIAG_CAPTURE_DURATION` definitions at the bottom of `main.rs`.
- [x] REMOVE IMPL: `rebuild_count` atomic and its plumbing in `render/mod.rs`.
- [x] REMOVE IMPL: any imports left unused after the above (compiler will surface them).
- [x] UPDATE VERSION: bump `Cargo.toml` patch `0.9.25 → 0.9.26`.
- [x] VERIFY: build and smoke test — `\` does nothing, nudge and general playback behaviour unchanged.
- [x] REVIEW: end-of-build walk through the full diff with the user.

## Conclusion

Completed.
