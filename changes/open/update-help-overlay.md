# Update In-App Help Overlay

## Intent

The `render_keyboard_help` function in `src/render/mod.rs` is hardcoded Rust and has drifted from the keybindings reference in three ways:

1. **Right block layout** — repeats per-deck labels three times (`╭ +Slp    ╭ +Slp    ╭ +Slp`); the reference condenses this to a single label per row (`╭  ╭  ╭ +Slp`), which is cleaner and narrower.
2. **Legend format** — horizontal `[Shift]  [Bare]  [Space]` on the separator line; the reference uses a vertical ╭│╰ box on the right of the three footer rows, which ties the legend visually to the modifier rows.
3. **Pitch labels swapped** — `A -Ptch` / `Z +Ptch` and `╰ -Ptch` / `╰ +Ptch` on the Space layer; correct values are `A +Ptch` / `Z -Ptch` and `╰ =Ptch` / `╰ =Ptch` (matching `pitch_up = "a"`, `pitch_down = "z"`, `pitch_reset = ["space+a", "space+z"]` in `config.toml`).

Update the overlay to match `keybindings.md`.
