# FPS Control

## Intent

The frame rate cap is only adjustable by editing `config.toml` — there is no way to change it during a session. Users experimenting with display smoothness must restart the application to see the effect of a different value.

## Approach

Two new actions — `FpsIncrease` and `FpsDecrease` — step through a fixed discrete level set: `[15, 20, 24, 30, 45, 60, 90, 120, 240]`. Default bindings: `^` (Shift+6) for increase, `Y` (Shift+Y) for decrease.

The runtime `target_fps: u32` replaces the `Option<u32>` in `DisplayConfig`. It is initialised from the config file value if present, otherwise defaults to `120`. The `floor` duration in the frame timing block becomes `Duration::from_secs_f64(1.0 / target_fps as f64)` unconditionally. The `FpsIncrease`/`FpsDecrease` actions step through the level set by finding the current value and moving one position up or down, clamping at the ends.

The detail info bar gains an `fps:{current}/{budget}/{cap}` field — e.g. `fps:58/60/120`. `current` is measured from the actual inter-frame interval: `(1.0 / frame_start.duration_since(last_render).as_secs_f64()).round() as u32`, captured before `last_render` is updated each frame. `budget` is the zoom-adaptive target: `(1.0 / frame_dur.as_secs_f64()).round() as u32`. `cap` is the runtime `target_fps`. All three values are expected to converge under normal conditions; divergence signals zoom pressure or machine load. This three-value layout is an interim design to be refined once user testing reveals which values are actually useful.

The config file comment is updated to reflect that `target_fps` now accepts only values from the discrete level set; values not in the set are snapped to the nearest level on load.

`SPEC/render.md` requires two updates: the stale FPS range sentence (line 164) is replaced with a table showing zoom-adaptive FPS at a representative screen width, and the spectrum analyser section gains an explicit statement that its update cadence is independent of the main frame rate. `SPEC/config.md` requires the keyboard diagram and global keys list to be updated to reflect the new bindings, and keys `^` and `Y` removed from the intentionally-unbound note.

Review cadence: at the end.

## Plan

- [x] UPDATE IMPL `config/mod.rs` — change `target_fps: Option<u32>` to `target_fps: u32` in `DisplayConfig`; update `Default` impl to `120`; update config parsing to snap any loaded value to the nearest entry in the discrete level set `[15, 20, 24, 30, 45, 60, 90, 120, 240]`; define the level set as a named constant
- [x] ADD IMPL `config/mod.rs` — add `FpsIncrease` and `FpsDecrease` variants to the `Action` enum and `ACTION_NAMES` table
- [x] UPDATE IMPL `resources/config.toml` — add default bindings `fps_increase = "^"` and `fps_decrease = "Y"` under `[keys]`; update the `target_fps` comment to note it is snapped to the nearest discrete level on load
- [x] UPDATE IMPL `main.rs` — remove the `Option` branch from the `floor` computation; `floor` is always `Duration::from_secs_f64(1.0 / display_cfg.target_fps as f64)`; change `display_cfg.target_fps` to a mutable local so the actions can update it at runtime
- [x] ADD IMPL `main.rs` — handle `FpsIncrease` and `FpsDecrease` in the event loop, stepping through the level set constant and clamping at the ends
- [x] ADD IMPL `main.rs` — compute three fps values each frame: `current_fps` from `frame_start.duration_since(last_render)` (captured before `last_render` is updated), `budget_fps` from `frame_dur`, and `cap_fps` from the runtime `target_fps`
- [x] UPDATE IMPL `main.rs` — add `fps:{current_fps}/{budget_fps}/{cap_fps}` to the detail info bar format string alongside the existing `zoom` and `lat` fields
- [x] UPDATE SPEC `SPEC/render.md` — replace the stale FPS range sentence with a zoom-adaptive FPS table (representative 200-column screen; columns: zoom level, natural FPS, effective FPS at max 120); add a sentence to the spectrum analyser section stating that its update cadence is driven by BPM and is independent of the main frame rate
- [x] UPDATE SPEC `SPEC/config.md` — update the keyboard diagram to show `^` on key `6` (Shift row) and `Y` on key `Y` (Shift row); remove the intentionally-unbound sentence entirely; add `fps_increase`/`fps_decrease` to the global keys list and legend
- [x] REVIEW read through change document and all edited files for consistency

## Log

User: fps display updates too fast to read — slow to once per second using an average.

User: when the fps cap is changed via key, update `fps_display.2` immediately rather than waiting for the next 1-second window.

Bug: pressing Esc while playing shows the red quit-confirmation bar only briefly. Root cause: Kitty injects a duplicate Esc Press event immediately after the first; `suppress_quit_until` absorbs duplicates for overlay-close paths but was not set when activating `pending_quit`, so the duplicate hit the quit confirmation intercept and immediately cancelled it. Fix: set `suppress_quit_until` when activating `pending_quit`, and check it in the intercept before processing.

Approach: accumulate a frame count over a rolling 1-second window. Each frame increments the counter; when the window elapses, compute `current_fps = frames / elapsed_secs`, snapshot `budget_fps` and `cap_fps`, reset the counter and window start. The displayed values are held stable between updates. `budget_fps` and `cap_fps` are sampled at update time (they change only on key press, so no averaging needed).

## Conclusion

Delivered as planned with the following additions surfaced during the build: fps display values update at most once per second using a frame-count average; the cap value updates immediately on key press; the Esc quit-confirmation bar bug was fixed (Kitty duplicate Esc was cancelling `pending_quit` immediately — fixed by setting `suppress_quit_until` on activation, consistent with the same mechanism used on overlay-close paths). `target_fps` in `DisplayConfig` is now a plain `u32` (default 120) with load-time snapping to the discrete level set; `FpsIncrease`/`FpsDecrease` actions step through that set at runtime; the detail info bar shows `fps:current/budget/cap`; SPEC files updated. Version bumped `0.9.17 → 0.9.18`.
