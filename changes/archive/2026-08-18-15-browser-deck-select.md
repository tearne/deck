# Browser Deck Select

**Mode:** Formal

*(Spun off from 10-keymap-rework.)*

## Intent

`Alt+j`/`Alt+k` should also cycle deck selection while the browser is open — deck switching shouldn't stop at the browser's door now the chords exist. Alongside, reconsider which deck the browser targets by default when opened (today: an empty deck, else a loaded-but-paused one, else the selected deck) — with selection cycling available everywhere, the target could follow the selected deck more directly, or the defaulting rule may simplify.

## Approach

Model U — the browser's floating target dies; there is only the selected deck.

- `Enter` (browser or context panel) loads to the **selected deck**; the chip shows it. `Alt+j`/`Alt+k` cycle selection identically inside and outside the browser, intercepted ahead of browser key handling so search mode never sees them.
- The least-disruptive default and the target keys (`[`/`]`, `1`/`2`/`3`) retire — opening the browser changes nothing silently, and the load-into-playing confirmation remains the guard.
- The accent bar stops being suspended while the browser is open: selection is now live there, so its indicator should be too.
- Map catch-up post-build: the Load Target node dissolves; Browser and Deck Selection absorb the story.

## Plan

- [x] `Alt+j`/`Alt+k` selection cycling reaches the browser and panel key paths
- [x] `Enter` loads to the selected deck; target concept and its keys removed; chip shows selection
- [x] Accent bar visible while the browser is open
- [x] keybindings.md updated

Added after first hand-back:

- [x] Deck chip coloured by landing zone — yellow when the selected deck isn't playing, red when it is
- [x] Load confirmation matches the quit confirmation: "load?  [y] load   [Esc/n] cancel", `y` or Enter accepted
- [x] Accent bar goes red (matching the chip) while the browser is open and the selected deck is playing
- [x] Load confirmation accepts only `y` — Enter dismisses, since Enter is the key that raised the prompt
- [x] Hot accent bar brightened: solid bright-red █ column instead of the muted thin line
- [x] Quit confirmation also goes `y`-only — it advertised [y] but silently accepted Enter
- [x] BPM confirmation likewise; all three warning prompts now route through one `confirms_destructive` helper carrying the rule
- [x] Chip red brightened to match the hot accent bar (255,70,70)

## Log

- The deck-number yellow highlight in the title corners was also un-suspended alongside the accent bar — both selection indicators now stay live while browsing.

- The `Alt+j`/`Alt+k` intercept sits after the tag-editor and load-confirm intercepts, so a pending confirmation still swallows every key as before.

- The chip's colour now signals load safety (yellow/red by playing state) rather than carrying the browser mode's accent; the mode accent remains on the borders and status bar.

- The keybindings.md confirmations table's `y`/`Enter` row needs a wrap-up correction — Enter no longer confirms anywhere.

- `[`/`]` and `1`/`2`/`3` were never documented in keybindings.md's browser table, so removal cost no doc rows; the chip and Enter descriptions updated.

## Conclusion

Completed at v0.25.8; minor bump confirmed. Eight hand-back refinements hardened the confirmations into a single `y`-only rule (`confirms_destructive`). Map updated at wrap-up: Load Target dissolved into Browser; the prompt convention recorded in Messages.
