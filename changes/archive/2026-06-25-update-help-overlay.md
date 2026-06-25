# Update In-App Help Overlay

**Mode:** Formal

## Intent

The `render_keyboard_help` function in `src/render/mod.rs` is hardcoded Rust and has drifted from the keybindings reference in three ways:

1. **Right block layout** — repeats per-deck labels three times (`╭ +Slp    ╭ +Slp    ╭ +Slp`); the reference condenses this to a single label per row (`╭  ╭  ╭ +Slp`), which is cleaner and narrower.
2. **Legend format** — horizontal `[Shift]  [Bare]  [Space]` on the separator line; the reference uses a vertical ╭│╰ box on the right of the three footer rows, which ties the legend visually to the modifier rows.
3. **Pitch labels swapped** — `A -Ptch` / `Z +Ptch` and `╰ -Ptch` / `╰ +Ptch` on the Space layer; correct values are `A +Ptch` / `Z -Ptch` and `╰ =Ptch` / `╰ =Ptch` (matching `pitch_up = "a"`, `pitch_down = "z"`, `pitch_reset = ["space+a", "space+z"]` in `config.toml`).

Update the overlay to match `keybindings.md`.


## Approach

One file: `src/render/mod.rs`, function `render_keyboard_help`. Three categories of change:

- **Content** — replace all row strings to match `keybindings.md`: condensed right block, fixed pitch labels, `╰ SpRst` at Space+C, vertical ╭│╰ legend appended to separator and footer rows
- **Dimensions** — recalculate `TEXT_W` to match the new longest line; `TEXT_H` stays 15
- **Colour coding** — preserve the existing `sh`/`ba`/`sp`/`gr`/`wh` scheme applied to the same semantic categories


## Conclusion

Completed. `render_keyboard_help` updated to match `keybindings.md`: condensed right block, fixed pitch labels (`A +Ptch`, `Z -Ptch`, `╰ =Ptch`), `╰ SpRst` at Space+C, vertical ╭│╰ legend. `TEXT_W` reduced from 87 to 78. Two post-build tweaks during review: `+Slp`/`-Slp` expanded to `+Slope`/`-Slope` for clarity, and `│ [Bare]` realigned by one space to match the column of `╭` and `╰`.


## Plan

- [x] Rewrite `render_keyboard_help` rows to match `keybindings.md`; update `TEXT_W`
