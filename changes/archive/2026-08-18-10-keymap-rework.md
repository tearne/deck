# Keymap Rework

**Mode:** Formal

*(Part of the loop-rethink sequence — see the design for ordering and context.)*

## Intent

Space becomes the reset modifier over the mixer keys, replacing the Alt-chord resets and freeing the whole Alt layer over those keys for features. `Alt+j`/`Alt+k` cycle the selected deck Zellij-style; the old selectors (`Alt+1/2/3`, `Alt+arrows`) are dropped, freeing those chords too. First of the sequence so the muscle memory settles early.

Design: [loop-rethink](../archive/2026-08-15-loop-rethink.md).

## Approach

### Full restore, fresh Alt

Space and Alt become two distinct chord layers (today `space+x` is parsed as an alias of `alt+x`). Every currently-chorded action returns to `space+…` — browser, play/pause, playlist skip, detect, metronome, swaps, resets, level max/min — restoring the pre-Alt layout. The Alt layer starts empty and is populated deliberately: `alt+j` next deck, `alt+k` previous, clip vocabulary later. The direct selectors (`alt+1/2/3`, `alt+arrows`) are dropped, not moved.

### Config compatibility

`alt+…` strings still parse and bind the Alt layer, so a user config keeps meaning what it says; only the shipped defaults change. A stale `--local-config` will keep the old Alt bindings wholesale — flagged at hand-back.

### Labels follow the bindings

The chord formatter learns `Space+X`, so the startup hint stays honest by construction; the help overlay's chord rows relabel `[Alt]` → `[Space]` and the new Alt residents are shown; keybindings.md is rewritten to match.

## Plan

- [x] Distinct Alt and Space chord layers in parsing and key handling; the alias removed
- [x] Defaults rewritten: all chords to `space+…`; `alt+j`/`alt+k` deck cycling; old selectors dropped
- [x] Chord naming: `Space+X` in the reverse-lookup formatter
- [x] Help overlay: `[Space]` relabel, Alt additions shown
- [x] keybindings.md updated

## Log

- The `select_deck1/2/3` actions were deleted outright, not just unbound — cycling is the only selection now, so the action names would have been dead config vocabulary.

- The chord formatter prefers Alt over Space when both layers bind an action, so future Alt residents advertise themselves in hints.

- Hand-back fix: the footer legend's ╭│╰ box lost column alignment when [Alt] grew to [Space]; all three rows now put the bracket at column 70, width 79.

- The cue-play warp-guard check (`space_held || alt` against the old unified chord) now checks each layer against its own binding.

## Conclusion

Completed at v0.24.1; minor bump confirmed. Spun off `15-browser-deck-select` (deck cycling inside the browser, and the load-target default). The Keymap map node was updated at wrap-up: Space chords are one-time actions because Space can't be reliably held; Alt is the sparse new-feature layer.
