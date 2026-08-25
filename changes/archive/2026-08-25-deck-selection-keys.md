# Deck Selection Keys

**Mode:** Wander

## Intent

Alt+j/k deck cycling works but doesn't feel natural. Augment with Alt+1/2/3 for direct selection of decks 1, 2 and 3. Related: [default-deck-count](default-deck-count.md) — the same Alt+number keys could summon/toggle decks. (2026-08-24)

## Conclusion

Three actions (`select_deck_1/2/3`, default `alt+1/2/3`) dispatching at deck level and inside the browser, alongside the cycling pair. Help footer reads `Alt+j/k/1/2/3 deck` (separators tightened to keep the box width). Shipped as 0.33.14. Groundwork noted for default-deck-count: Alt+number can later also summon decks.
