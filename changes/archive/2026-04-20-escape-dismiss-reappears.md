# Escape Dismiss Reappears

## Intent

Pressing escape during playback displays the red warning as expected, but pressing escape again to dismiss it makes the warning go away then immediately reappear.

## Approach

**Root cause.** When the warning is shown (`pending_quit = Some`) and the user presses Esc to dismiss, the cancel branch of the quit-confirmation intercept (`src/main.rs:1322-1340`) clears `pending_quit` but does not set a fresh `suppress_quit_until`. The original `suppress_quit_until` (set by `Action::Quit` when the warning first appeared) is consumed by `.take()` on entry to the intercept. If a second Press event for the same dismiss-Esc keystroke arrives — which happens under crossterm + Kitty, where key-repeats can decode as additional Press events (existing comment at `src/main.rs:1147` calls this out for the space-modifier path) — it now sees `pending_quit = None` and no suppression, falls through to `Action::Quit`, and re-arms the warning.

This matches the symptom exactly: warning disappears (cancel branch fires), then immediately reappears (followup Press re-enters Action::Quit).

**Fix.** In the cancel branch (the `continue 'tui;` at the end of the `if pending_quit.is_some()` block), set `suppress_quit_until = Some(Instant::now() + Duration::from_millis(300));` before continuing — mirroring the help-overlay dismiss (line 1304) and the notification dismiss (line 1345), which both already do this for the same reason.

**Verification.** Manual: play a track, press Esc, confirm warning appears; press Esc again, confirm it stays dismissed and does not re-arm.

**Map impact.** None — bug fix in input dispatch, no mapped concept changes.

**Version.** Patch bump for the testable build.
**Review cadence.** End-of-build.

## Plan

- [x] UPDATE IMPL: in the quit-confirmation cancel branch in `src/main.rs`, set `suppress_quit_until = Some(Instant::now() + Duration::from_millis(300))` immediately before the trailing `continue 'tui;`
- [x] UPDATE Cargo.toml: patch bump
- [x] BUILD: release build; user verifies the dismiss now sticks
- [x] REVIEW: end-of-build walk-through with user

## Conclusion

Completed.

