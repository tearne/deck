# Keymap: Fuzzy Finder on Space+F

**Mode:** Formal

## Intent

Honour Helix muscle memory: `space+f` should open the fuzzy finder (`open_browser`), matching how the user already reaches for it in Helix. Currently `space+f` is bound to `play_pause`, and `open_browser` sits on `space+d`.

Freeing `space+f` requires relocating `play_pause`, which in turn collides with the existing `loop_exit` binding on `space+g`. The full resolution is a four-way rotation, decided through discussion:

| Action | Current | New |
|---|---|---|
| `open_browser` | `space+d` | `space+f` |
| `play_pause` | `space+f` | `space+g` |
| `pfl_on_off` | `shift+g` (`G`) | `space+d` |
| `loop_exit` | `space+g` | `shift+g` (`G`) |

Rationale: Play is a rare action and doesn't need a natural-resting-finger key. PFL stays on a space modifier per preference. Loop keys are relocated, not removed — the loop feature stays as-is, including the separately tracked `loop-bounds-atomic-race` fix.

Also discovered along the way: the in-app help overlay (`src/render/mod.rs:1835`) mislabels `pfl_on_off` as a space+G action (`PFLTog` sits in the space row for the G column) when it's actually bound to bare `shift+g`. This row needs correcting regardless, since the rotation changes what all of d/f/g show — fold the fix in as part of updating the overlay.

## Approach

### Change scope: default keymap only

Only `resources/config.toml`'s defaults move. `Action`/`ACTION_NAMES` in `src/config/mod.rs` are unaffected — no action is renamed or added, only which key string each maps to.

### Overlay: loop stays hidden, PFL/browser/play get correct cells

The overlay currently omits `loop_tap` (bare `g`) entirely — it's not shown in the quick-reference grid, only documented in `keybindings.md`'s separate experimental Loop section. Keep that convention until the loop feature actually lands: `loop_exit`'s new home (`shift+g`) also stays out of the main overlay grid, so the G column's shift/bare cells remain blank as today. Only the space row changes: D's space cell becomes `pfl_on_off`'s label, F's space cell becomes `Brows`, G's space cell becomes `Play` — replacing the current, already-incorrect `PFLTog` placement.

## Plan

- [x] Update `resources/config.toml` default key bindings for `open_browser`, `play_pause`, `pfl_on_off`, `loop_exit`
- [x] Update `keybindings.md` overlay diagram and per-action tables for the four rebound actions
- [x] Update `README.md` overlay diagram for the four rebound actions
- [x] Fix the D/F/G space-row cells in the `src/render/mod.rs` help overlay (Brows/Play/PFL placement)
- [x] Shift `cue` (`space+e` → `space+r`) and `cue_play` (`space+r` → `space+t`) to restore vertical pairing with the new Browse/Play columns, in `config.toml`, `keybindings.md`, `README.md`, and the `render/mod.rs` overlay
- [x] Superseding the above: move `cue` to `shift+b` and `cue_play` to `shift+g` instead, so the operator can hold shift and repeatedly jump to the cue point; relocate `loop_exit` from `shift+g` to `shift+h` to free the slot, across `config.toml`, `keybindings.md`, `README.md`, and the `render/mod.rs` overlay

## Log

- Did the `render/mod.rs` overlay task before the two markdown docs (out of plan order) so the pixel-exact column widths could be computed once and copied verbatim into `keybindings.md` and `README.md`, rather than hand-realigning the ASCII grid three times.
- User spotted a knock-on inconsistency during testing: the rotation broke the vertical pairing between transport keys (D/F/G row) and cue keys (E/R/T row directly above). Added a task to shift the cue keys right by one column to restore it.
- Config purge issue found during testing: `~/.config/deck/config.toml` is only auto-created once and never refreshed from the embedded defaults, so a pre-existing user config on the test host was masking all of today's rebinds. Not a code bug — resolved by deleting the stale file. `dev-build-run.sh` already does this automatically; noted here since it cost a debugging round.
- User then asked for `cue`/`cue_play` on `shift+b`/`shift+g` for repeatable shift-held cue jumping, which re-collided with `loop_exit` (placed on `shift+g` earlier in this same change). Resolved by moving `loop_exit` to `shift+h`, which also has the nice side effect of taking it off the visible overlay grid entirely (H isn't part of any grid column), rather than requiring a deliberate "stays hidden" carve-out as before.

## Conclusion

Final keymap: `open_browser`→`space+f`, `play_pause`→`space+g`, `pfl_on_off`→`space+d`, `cue`→`shift+b`, `cue_play`→`shift+g`, `loop_exit`→`shift+h`. Shipped as `v0.10.2` — the minor bump held; scope grew through two follow-on tweaks (cue-key repositioning, twice) but stayed within the same default-keymap nature. Touched `resources/config.toml`, `keybindings.md`, `README.md`, the `src/render/mod.rs` help overlay, and `Cargo.toml`/`Cargo.lock`. No project changelog exists, so no entry was added. Full back-and-forth is in the Log above.

