# Deck-Independent Browser

**Mode:** Formal

## Intent

The browser is opened *for* a specific deck (the selected one at open time) and loading a track sends it to that deck. But the browser's other operations — rename, move, and the compliance work to come — are file operations with nothing to do with a deck. So the browser now does deck-agnostic file work while still being "owned" by a deck, which feels odd.

Make the browser independent of any deck. Loading is the only deck-specific action, so the **load action chooses the target deck at load time** — with the usual confirmation when the chosen deck is already playing — rather than the deck being fixed when the browser opens.

Captured as a follow-on to [[browser-file-operations]]; not yet scoped.
