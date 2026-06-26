# Rename redetect_bpm

**Mode:** Formal

## Intent

The config action `redetect_bpm` implies BPM detection happens automatically on load and this is a re-run. In practice, detection is only ever manually triggered — the name is a vestige of removed behaviour. Rename to `detect_bpm` for accuracy.


## Approach

Five touch points, no open decisions: `src/config/mod.rs` (enum variant + action-name string), `src/main.rs` (Action reference), `resources/config.toml` (key binding entry), `keybindings.md` (table entry), `map.md` (Beat Grid node prose).


## Conclusion

Completed. `redetect_bpm` renamed to `detect_bpm` across all five touch points: enum variant, action-name string, config.toml, keybindings.md, map.md.


## Plan

- [x] Rename `RedetectBpm` → `DetectBpm` in `src/config/mod.rs` (enum variant and action-name string)
- [x] Update `Action::RedetectBpm` → `Action::DetectBpm` in `src/main.rs`
- [x] Rename `redetect_bpm` → `detect_bpm` in `resources/config.toml`
- [x] Update `keybindings.md` table entry
- [x] Update `map.md` Beat Grid node
