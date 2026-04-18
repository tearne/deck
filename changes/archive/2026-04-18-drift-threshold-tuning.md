# Drift Threshold Tuning

## Intent

The drift-snap threshold is currently 0.3 s — chosen as a value above the typical steady-state drift on a loaded system but below a single beat at any practical BPM. With the nudge-tracking fixes now in place, periodic drift build-up from nudge events should be largely gone, so the threshold can probably be tightened without triggering visible snaps during normal playback.

The user is curious whether a smaller threshold produces visibly tighter lockstep between the displayed waveform and what they hear, and where the practical lower bound sits before snaps start showing up during ordinary use.

## Approach

**Interactive experimentation.** Rather than picking a value, rebuilding, and iterating, wire up a temporary cycle on the `\` key. Each press steps through the candidate list and shows the current value as a transient global notification. The user can feel the difference in real time and land on a preferred value.

**Candidate values.** Cycle through `0.3 → 0.1 → 0.05 → 0.02` and wrap around. Starts at 0.3 s (current behaviour).

**Paused-snap threshold and 0.002 correction factor are out of scope.** The snap threshold for the paused-and-not-nudging case stays at one sample. The partial drift correction factor is tracked separately in `partial-drift-correction-factor.md`.

**Scaffolding and cleanup.** The cycle key, candidate list, and notification-on-change are temporary. After the user picks a value, this change hardcodes it in place of the current literal `0.3` and removes the scaffolding — all within this change.

**Notification.** Reuses the existing `global_notification: Option<Notification>` infrastructure and `NOTIFICATION_TIMEOUT` (5 s). Message format: `drift threshold: 0.10 s`.

**Version bumps.** Patch bump on `Cargo.toml` for each testable build the user will run, per the repeating pattern.

**Review cadence.** End-of-build reviews (scaffold build, then cleanup build), not per task.

## Plan

**Scaffold build**

- [x] ADD IMPL: mutable threshold state in the main event loop (e.g. an index into the candidate list, or the value itself), starting at 0.3 s to preserve current behaviour.
- [x] ADD IMPL: `\` key handler cycles through `[0.3, 0.1, 0.05, 0.02]` with wrap-around.
- [x] ADD IMPL: on cycle, set `global_notification` to a transient message showing the new value (e.g. `drift threshold: 0.10 s`) with `Instant::now() + NOTIFICATION_TIMEOUT`.
- [x] UPDATE IMPL: replace the hardcoded `0.3` in the drift-snap check (`service_deck_frame`) with the current threshold state.
- [x] UPDATE VERSION: bump `Cargo.toml` patch for the scaffold build.
- [x] REVIEW: end-of-build walk through the scaffold diff with the user.
- [x] VERIFY: user experiments live and settles on a preferred value.

**Cleanup build**

- [x] UPDATE IMPL: replace the threshold state read in the drift-snap check with the chosen hardcoded literal.
- [x] REMOVE IMPL: `\` key handler, threshold state, candidate list, and cycle-notification code.
- [x] UPDATE MAP: if the chosen value differs from 0.3 s, update the Drift Snap node's Detail bullet for the playing-mode threshold.
- [x] UPDATE VERSION: bump `Cargo.toml` patch for the cleanup build.
- [x] REVIEW: end-of-build walk through the cleanup diff with the user.
- [x] VERIFY: smoke test — drift behaviour matches the chosen value during the scaffold run.

## Conclusion

Experiment landed on the pre-existing 0.3 s threshold. Candidate list was extended mid-scaffold at the user's request to include 0.5 s and 1.0 s above the original range (`[0.02, 0.05, 0.1, 0.3, 0.5, 1.0]`). Map unchanged since the chosen value matched the existing Drift Snap node Detail.
