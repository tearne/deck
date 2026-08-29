# Browser Location Key

**Mode:** Wander

## Intent

In the browser, `` ` `` cycles a fixed list of locations — working directory, then each deck's track in slot order — regardless of which deck is selected. That made sense when decks were hard to address; now that `Alt+j`/`Alt+k` cycle the selected deck everywhere, including inside the browser, the operator's expectation is "take me to the selected deck's track", and the first press landing on deck 1's file instead reads as a wrong jump. Decided 2026-08-29: `` ` `` becomes a single-fire jump to the selected deck's track — no cycle, no location list. `Alt+j`/`Alt+k` change which deck that is. The cycle's "Working directory" home stop disappears with it.

Observed 2026-08-22: after moving deck 1's file and loading a new track on another deck, the first press went to the moved file in its new folder.

## Conclusion

`` ` `` is a single fire: `go_to` the selected deck's track, a notification on an empty deck; the location list and its rotation state are deleted. The working-directory home stop went with the cycle, accepted at Intent. Shipped as 0.34.5. Map impact: the Jump to Loaded node.
