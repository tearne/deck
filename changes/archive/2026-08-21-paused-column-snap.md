# Paused Column Snap

**Mode:** Formal

## Intent

*(Captured as an aside during 30-grid-anchors map catch-up.)*

The detached view's whole-column snap (no sub-column smoothing when nothing moves) proved better for precision viewing than the half-column form. Apply the same snap whenever a deck is paused: the smoothing exists for scroll motion, and a stationary waveform gains nothing from half-column placement — while nudge-scrubbing a paused deck suffers the same registration ambiguity the detached view did.

## Approach

Snapping means locking the displayed position onto the renderer's grid — deterministic rendering, nothing left to per-frame rounding — and each static state snaps to the finest grid its overlays allow: the detached view to character cells (its glyph markers are character-sized), a paused deck to braille dots (no overlays; the cell's native half-column resolution). Playing decks keep continuous smoothing.

Nudge steps become zoom-adaptive so display and audio can never disagree: paused moves one dot of audio per press, the detached cursor one character — a press is always exactly one visible step, and scrub precision follows zoom (finer zoomed in, coarser out). The fixed 10 ms paused step retires. Ticks and marks extract from the snapped position, so their parity is constant by construction.

## Plan

- [x] Paused decks snap to the dot grid; detached stays on the character grid
- [x] Zoom-adaptive nudge: one dot per press paused, one character detached

## Log

- Scrub bursts gained a 35–80 ms length window (floor for audibility, cap against key-repeat stacking when zoomed out), 5 ms edge fades, and a 0.65 gain trim so overlapping bursts can't clip: at fine zoom a one-column snippet was a bare click, and unfaded edges ticked loudly on every press. Surfaced by the zoom-adaptive steps making fine-zoom scrubbing common.

## Conclusion

Completed at v0.29.13; patch bump confirmed. The change grew from a display snap into the zoom-adaptive nudge model (one dot per press paused, one character detached — display and audio can never disagree) plus scrub-audio polish: 35–80 ms burst window, 5 ms edge fades, 0.65 gain trim. Map: Nudge and Sub-Column Smoothing touched at wrap-up.
