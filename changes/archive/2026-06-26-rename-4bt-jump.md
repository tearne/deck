# Rename jump_forward_4bt / jump_backward_4bt

**Mode:** Formal

## Intent

The config actions `jump_forward_4bt` and `jump_backward_4bt` describe the jump as "4 beats" but 4 beats = 1 bar, and the in-app overlay already labels key 2 as `+1b` (1 bar). Rename to `jump_forward_1b` / `jump_backward_1b` so the action names match the overlay labels and the bar-unit convention used by all the larger jumps.


## Approach

Five touch points, no open decisions: `src/config/mod.rs` (two enum variants + two action strings), `src/main.rs` (two Action references), `resources/config.toml` (two keys), `keybindings.md` (two table entries). `map.md` Beat Jump node already says "1 bar" — no change needed.


## Conclusion

Completed. `jump_forward_4bt`/`jump_backward_4bt` renamed to `jump_forward_1b`/`jump_backward_1b` across all five touch points, consistent with the in-app overlay label and the bar-unit convention used by all larger jump sizes.


## Plan

- [x] Rename `JumpForward4bt`/`JumpBackward4bt` → `JumpForward1b`/`JumpBackward1b` in `src/config/mod.rs`
- [x] Update `Action::JumpForward4bt`/`Action::JumpBackward4bt` in `src/main.rs`
- [x] Rename `jump_forward_4bt`/`jump_backward_4bt` in `resources/config.toml`
- [x] Update `keybindings.md` table entries
