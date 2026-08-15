# Clip Persistence

**Mode:** Formal

*(Part of the loop-rethink sequence — see the design for ordering and context.)*

## Intent

Where grid, clips, and per-track mode memory live. First choice to explore: embedded in the track's own metadata, travelling with the file (tag writes are identity-safe by design). Fallback: the track database, like cue and gain today.

Design: [loop-rethink](../archive/2026-08-15-loop-rethink.md).
