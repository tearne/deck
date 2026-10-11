# Keyboard Zoning

**Mode:** Formal

## Intent

Bake the accepted keyboard-zoning preview into the default bindings and help overlay: selected-deck controls in columns 1–4, shared controls in 5–6, three-deck mixer controls on the right, and a visible Alt layer. Include jumps through 128 bars and the agreed cue, reset, view, mode, and metronome placements.

## Accepted design

2026-09-20: After iterating previews 00–26, the user asked to bake in version 26. Version 26 was the accepted visual reference; the final layout, including the later backtick note, lives in [the renderer](../../src/render/mod.rs) and [its fixture](../../tests/fixtures/keyboard-help.txt). Three decks remain; the four-deck experiment was backed out.

## Approach

Apply the accepted defaults and reproduce the preview's spacing, colours, four layers, extended keys, and corner legend in the help overlay. Add ±128-bar jump actions to the existing shared jump machinery. Space+X uses the existing PFL toggle: active levels reset to zero/off, inactive PFL turns on at 100%. Keep the separately configurable reset action available without a default binding. Update the key reference and README, verify the defaults and rendering, and run the Rust tests/build. Preserve user configuration overrides and explain that existing configs can retain previous bindings.

## Worklist

- [x] Update default bindings and add ±128-bar jumps.
- [x] Render the accepted layout, including small-area clipping.
- [x] Update reference documentation and prepare any map amendments.
- [x] Check binding collisions, jump behaviour, rendering, tests, and build.

## Log

- 2026-09-20: Build authorized by “This can be baked in”; active lock taken. PFL reset was previously described too loosely: its actual default is 0/off, so existing `pfl_on_off` already provides the requested reset/toggle behaviour. No new PFL state machine is needed.

- 2026-09-20: Found browser command `h` already navigates to the parent directory. Preserve that fixed browser command; the new plain-h Help alias applies at the decks, while `?` remains Help in browser command mode.

- 2026-09-20: User approved relaxing the forward-jump boundary after Shift+4 worked only when paused. While playing, require only destination ≤ track end, not room for a second jump. Applied to Beat-mode jumps, forward Playback-mode jumps, and shared ghost landing checks; paused clamping and backward behaviour stay as before.

- 2026-09-21: Final inspection found the special nudge-mode handler still resolved plain keys only. Shared layer-aware lookup with ordinary actions, enabled the accepted Space+C binding, and consumed the chord to prevent fallthrough. Added a regression check for modifier precedence.

- 2026-09-21: User requested fixing the parked backtick omission before concluding. Added a browser-only jump-to-selected-track note in the spare help footer row, clarified the live contextual hint, and updated reference layouts and preview 27 without increasing dimensions.

## Map sync

- Beat Jump: approved and applied 2026-09-21, 770 characters including Detail. Includes 128 bars and the intentional boundary differences: preserve Beat-mode sync; allow Playback-mode navigation to the start; refuse forward overshoot to avoid ending playback.
- Keymap: approved and applied 2026-09-21, 797 characters. Explicitly describes the default keymap, visible layers, configurable bindings and fixed contextual input.

## Validation

- `cargo test --workspace --quiet`: 75 passed, 2 pre-existing ignored tests (including the forward-jump boundary regression).
- `cargo build --release`: succeeded; `target/release/deck --help` succeeded.
- `git diff --check`: clean.
- Renderer text and every non-space foreground colour match preview 27 (accepted layout plus the requested backtick note); small rendering areas do not panic.
- Default config has no duplicate bindings or unknown actions; shifted jumps cover 16/32/64/128 bars in both directions.
- Browser hints preserve h as navigation, with ? for Help; existing config overrides remain intact.

## Status

Implementation, reference docs and map sync complete. Conclude approved on 2026-09-21. No commit or release performed. The clip-persistence deletion predates this build and is unrelated.

- 2026-09-20: User identified contextual-help gaps after implementation: browser hjkl meanings and grid-session a (pin anchor). Parked separately in [Mode Shortcut Help](../open/mode-shortcut-help.md) for future work. Map wording approval remains pending; this feedback was not approval.

## Final tidy

2026-09-21: Checked both project maps (81 nodes): node IDs, parent navigation, contents coverage and local links. No structural or link corrections needed. Counted 24 nodes above 800 characters, with no approver exemptions. Context-dependent shortcut help remains parked separately; nothing held within this change.

2026-09-21: Removed all 28 exploratory preview scripts at the user’s request. The baseline renderer is in Git history; the final renderer and text/colour fixtures preserve the implemented layout.

## Conclude

Completed. Mode Shortcut Help remains parked as a separate change.
