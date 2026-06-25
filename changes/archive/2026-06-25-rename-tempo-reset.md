# Rename and Bind tempo_reset

**Mode:** Formal

## Intent

`tempo_reset` resets playback speed to nominal (`bpm` → `base_bpm`, `vinyl_speed` → 1.0) but has no default key binding and its name implies BPM detection rather than speed. Rename the config action to `speed_reset` for accuracy, give it a default binding (Space+C is a candidate — currently unbound on the Space layer), and add it to the keyboard layout diagram (`SpRst` label).


## Approach

Four touch points, no open decisions:

- `src/config/mod.rs` — rename `TempoReset` enum variant to `SpeedReset`; update the action-name string from `"tempo_reset"` to `"speed_reset"`
- `src/main.rs` — update `Action::TempoReset` → `Action::SpeedReset`
- `resources/config.toml` — add `speed_reset = "space+c"`
- `keybindings.md` — update the BPM & Beat Grid table entry; add `╰ SpRst` at Space+C in the layout diagram (currently blank on the ZXCV Space layer)


## Conclusion

Completed. `tempo_reset` renamed to `speed_reset` throughout (`src/config/mod.rs`, `src/main.rs`, `resources/config.toml`), bound to `space+c`, and `keybindings.md` updated with the new name, default key, and `╰ SpRst` label at Space+C. The `?` help overlay is not updated here — that's covered by `update-help-overlay.md`.


## Plan

- [x] Rename `TempoReset` → `SpeedReset` in `src/config/mod.rs` (enum variant and action-name string)
- [x] Update `Action::TempoReset` → `Action::SpeedReset` in `src/main.rs`
- [x] Add `speed_reset = "space+c"` to `resources/config.toml`
- [x] Update `keybindings.md`: rename table entry and add `╰ SpRst` at Space+C in the layout diagram
