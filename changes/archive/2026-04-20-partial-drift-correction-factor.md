# Partial Drift Correction Factor

## Intent

The partial drift correction (mapped in the Partial Drift Correction node) pulls each frame's rendered position 0.2% toward the audio's true position, acting as a low-pass filter on audio-batch step noise. Now that the user has a clearer mental model of the rendering pipeline, they're curious whether adjusting the 0.002 factor — up or down — yields visibly smoother scrolling.

## Approach

Same scaffold-and-cleanup shape as `drift-threshold-tuning`.

Candidate factors, log-spaced (each step ~2–2.5×):

`[0.0002, 0.0005, 0.001, 0.002, 0.005, 0.01, 0.02]`

Starting index: 3 (0.002, current behaviour).

Scaffold: the `\` key cycles through the candidate list; the global notification displays the current factor; the frame loop reads the factor from mutable state instead of the literal.

Cleanup (within this change): replace the state-read with a hard-coded literal for the chosen value, add a rationale comment (mirroring the 0.3s treatment), and strip the `\` handler, candidate list, and index state.

Version: patch bump per testable build (scaffold, any mid-experiment rebuilds, final cleanup). Review at the end, not per task.

## Plan

- [x] ADD IMPL: candidate list + index state; frame loop reads factor from state instead of literal
- [x] ADD IMPL: `\` key handler — cycle index, set notification showing current factor
- [x] UPDATE Cargo.toml: patch bump for scaffold build
- [x] BUILD: release build; user experiments live
- [-] UPDATE IMPL: hard-code chosen factor as literal with rationale comment
- [-] REMOVE IMPL: strip `\` handler, candidate list, index state
- [-] UPDATE map.md: if chosen factor differs from 0.002, update the Partial Drift Correction node's Detail line
- [-] UPDATE Cargo.toml: patch bump for cleanup build
- [-] BUILD: final release build
- [-] REVIEW

## Feedback

**Status:** not implemented.

**Notes:** across the candidate range `[0.0002, 0.0005, 0.001, 0.002, 0.005, 0.01, 0.02]`, the user could not see a visible difference in scrolling smoothness. A new hypothesis emerged during the experiment: the partial drift correction is currently absolute (applied to sample-count drift directly), but should likely be relative to the zoom level — expressed as a percentage of the samples represented by a single column character. The same rethink may apply to the drift-snap threshold. Scaffold reverted; code back to pre-change state at v0.9.29.

**Documentation impact:** if the correction is reworked to be zoom-relative, the Partial Drift Correction and (possibly) Drift Snap nodes in `map.md` will need their Detail sections updated.

## Conclusion

Closed without code change. The zoom-relative follow-up (`changes/archive/2026-04-20-zoom-relative-drift-handling.md`) tested three correction forms across their candidate ranges and also produced no visible difference, so the 0.002 baseline is twice-confirmed. The source comment on the 0.002 line was extended in that follow-up to record both findings, sparing future readers from repeating either experiment. Map unchanged.
