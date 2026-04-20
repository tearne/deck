# Zoom-Relative Drift Handling

## Intent

The partial drift correction factor (0.002) is currently absolute, applied against raw sample-count drift without reference to what the user can actually see. The visible impact of drift is a function of how many samples a single column represents — so a fixed absolute correction over-reacts at one zoom and under-reacts at another.

The user would like to reframe the partial drift correction as **zoom-relative**: expressed as a fraction of the samples represented by a single column character. The hypothesis is that tying correction to visible motion will yield smoother scrolling across the full zoom range and make the knob easier to reason about.

The drift-snap threshold is left as-is. It governs audio-visual sync — a time quantity, not a visible-motion one — so the existing 0.3s absolute framing is the right shape.

Supersedes `partial-drift-correction-factor.md`, whose absolute-factor experiment produced no visible difference across its candidate range.

## Approach

**Scope.** Partial Drift Correction only. Reframed as a fraction of one column pulled per frame (capped by the actual drift). Snap threshold left at 0.3s — it's an audio-visual sync concern, not a visible-motion one. Reading `samples_per_col` from the per-deck `BrailleBuffer` mirrors how the scrub block already reads it (`src/main.rs:1984`).

**Where the change lands.** The `else if` branch of the drift block in `src/main.rs:2019-2024`. No new state, no signature changes.

**Three candidate forms** scaffolded behind a form switch so the user can compare them live:

- **(a) Constant chase.** `correction = drift.signum() * samples_per_col.min(|drift|) * cols_per_frame`. Fixed column-speed toward audio, capped by remaining drift. Knob: `cols_per_frame`.
- **(b) Hybrid (decay with column cap).** `correction = drift.signum() * (|drift| * k).min(samples_per_col * cols_per_frame)`. Today's exponential decay below the cap, column-bounded above it. Primary knob: `cols_per_frame`; secondary `k` held at today's 0.002.
- **(c) Baseline / control.** `correction = drift * k`. Today's absolute-factor behaviour, included explicitly as a control so the zoom-relative forms can be A/B'd against it. Knob: `k`.

**Scaffold-and-cleanup.** Same shape as the previous drift experiment. Two cycle keys — one for partial-correction form (a/b/c), one for the active form's primary knob. Notification displays the current form and value. Frame loop reads from state. After settling: hard-code the chosen form and knob, comment with rationale, strip scaffold (including the two unused forms).

**Candidate ranges and starting indices.**

- Forms a and b `cols_per_frame`: `[0.001, 0.002, 0.005, 0.01, 0.02, 0.05]`. Starting index 2 (0.005).
- Form c `k` (baseline): `[0.0005, 0.001, 0.002, 0.005, 0.01, 0.02]`. Starting index 2 (0.002, today's value).
- Form switch starts at (b) — the safest baseline.

**Cycle keys.** `\` cycles the active form's primary knob value; `|` cycles the form (a/b/c).

**Map impact.** Only the Partial Drift Correction Detail section changes. Drift Snap is unchanged. Pre-staged:

*Partial Drift Correction (Detail section, full proposed content)*

```
**Detail**

- Per-frame correction pulls the rendered position toward the audio position, bounded by a fraction of one column per frame so visible motion stays consistent across zoom levels.
- Applies only while playing and below the snap threshold.
- The framing is column-relative because an earlier experiment with an absolute factor showed no visible difference across its candidate range — visible jerk depends on zoom, so the correction should too.
```

Final numbers slotted in after the experiment.

**Source comment on the final hard-coded value** will mirror this: note the chosen form and knob with rationale, and include the "tried absolute factor across [0.0002…0.02], no visible difference — reframed as zoom-relative" history line so a future reader doesn't re-run the same experiment.

**Version.** Patch bump per testable build (scaffold, mid-experiment rebuilds, final cleanup).
**Review cadence.** End-of-build.

## Plan

- [x] ADD IMPL: form enum (a/b/c) + knob state per form; frame loop reads form + knob from state
- [x] ADD IMPL: `\` handler — cycle active form's knob, notification shows form + value
- [x] ADD IMPL: `|` handler — cycle form, notification shows form + knob's current value for that form
- [x] ADD IMPL: `samples_per_col` read from the per-deck BrailleBuffer in the drift block
- [x] ADD IMPL: three correction forms (a/b/c) dispatched by form enum
- [x] UPDATE Cargo.toml: patch bump for scaffold build
- [x] BUILD: release build; user experiments live
- [x] REVERT IMPL: strip statics, key handler, and three-form dispatch; restore `d.display.smooth_display_samp -= drift * 0.002;`
- [x] UPDATE IMPL: extend the rationale comment on the restored line to note that both absolute and zoom-relative framings were tested and showed no visible difference
- [-] UPDATE map.md: not needed — no change to behaviour, so Partial Drift Correction Detail stays as-is
- [x] UPDATE Cargo.toml: patch bump for post-revert build
- [x] BUILD: final release build
- [x] REVIEW: end-of-build walk-through with user

## Conclusion

Three correction forms (constant chase, hybrid, absolute baseline) scaffolded behind form-switch and knob-cycle keys, tested live across their candidate ranges. No visible difference between forms or knob values. Combined with the prior `partial-drift-correction-factor` run, the 0.002 absolute factor is now twice confirmed as effectively optimal within the explored ranges — across both absolute and zoom-relative framings.

Scaffold reverted. The in-source comment on the 0.002 line extended to record the finding so a future reader doesn't repeat either experiment. Map unchanged.
