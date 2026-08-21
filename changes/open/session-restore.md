# Session Restore

**Mode:** Formal

## Intent

*(Under consideration — parked until wanted.)*

On startup, pressing some key could reload the last known tracks — the decks as they were when the player last quit — instead of starting empty and re-browsing for each deck.

2026-08-21: fresh motivation — accidentally exiting loses all session data, which is irritating enough that a general exit confirmation was considered and rejected as more friction. Automatic session save (with restore on startup) is the better shape; the design should treat protection against accidental exit as a primary goal, not just startup convenience.
