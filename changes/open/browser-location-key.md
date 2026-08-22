# Browser Location Key

**Mode:** Formal

## Intent

In the browser, `` ` `` cycles a fixed list of locations — working directory, then each deck's track in slot order — regardless of which deck is selected. That made sense when decks were hard to address; now that `Alt+j`/`Alt+k` cycle the selected deck everywhere, including inside the browser, the operator's expectation is "take me to the selected deck's track", and the first press landing on deck 1's file instead reads as a wrong jump. Reconsider what the key does: jump to the selected deck's track (with `Alt+j`/`Alt+k` then changing which track that is), a cycle that starts from the selected deck, or a different split between "home" and "deck" locations.

Observed 2026-08-22: after moving deck 1's file and loading a new track on another deck, the first press went to the moved file in its new folder.
