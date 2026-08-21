# Ghost Playheads

**Mode:** Formal

*(Part of the loop-rethink sequence — see the design for ordering and context.)*

## Intent

Subtle marks on the overview showing where each beat-jump key would land (e.g. ±8 bars), so a jump is aimed rather than counted. Beat mode only.

Design: [loop-rethink](../archive/2026-08-15-loop-rethink.md).

## Approach

### Both waveforms, each showing the landings that fit it

Every jump size is a candidate; a view draws a landing only when it falls inside that view's visible extent and clear of the playhead column. The overview naturally collects the large jumps and the detail waveform the small ones, with no fixed split — a short track or a wide zoom simply shifts which sizes appear where.

### A mark is drawn only where the key would land

Each mark is placed with the Beat Jump rules, including its refusals: backward landings clamp to the track start and forward jumps that would be refused near the end get no mark. The presence of a mark then means "this key works", which is the point of aiming.

### Beat mode only, following the playhead

Marks are derived from the live playhead; in Playback mode none are drawn. The detached grid-refinement cursor is not marked.

### The mark is the key

Each ghost is the single character of the key bound to that jump, read from the live keymap (so a rebound key relabels itself), drawn in violet — distinct from the magenta cue and the grey grid furniture. It occupies one cell on the view's middle row, where it never obscures a peak; playhead and cue keep precedence over it, and it displaces a bar marker or tick glyph in its column. Two landings sharing a column show the larger jump's key.

### Detail marks go through the shared sample-to-column mapping

Per the Grid Refinement callout, detail-view landings are positioned with the same buffer-anchored mapping the waveform and ticks use, never separate arithmetic.

### Part of the overview cache key

Ghost columns and their labels join the key that decides whether the overview is rebuilt, so the marks move with the playhead at the same cadence and cost nothing between moves.

## Plan

- [x] Move the jump-size table (action → beats) out of the key handler into a single shared definition used by jumps, the detached cursor, and ghosts
- [x] Compute ghost landings for a deck: per size and direction, the landing sample under Beat Jump rules, labelled with the bound key character; empty outside Beat mode
- [x] Add a violet ghost colour alongside the existing marker colours
- [x] Overview: draw ghost labels on the middle row, behind playhead and cue, ahead of bar markers; collapse shared columns to the larger jump; add ghost columns and labels to the overview cache key
- [x] Detail: map landings to screen columns via the shared sample-to-column mapping and draw labels on the middle row, behind playhead and cue, ahead of tick glyphs
- [x] Pass the keymap to the two render paths
- [x] Hand back for visual check (violet tone, label legibility on both views)

## Log

- Render paths take the computed landings rather than the keymap: main computes `ghost_landings` once per deck and hands it to both views, so the detached-view case (detail gets an empty list) is decided in one place.
- Tick glyphs live in the shared tick row, not inside the detail waveform rows, so there is no tick-vs-ghost precedence to settle there — the ghost only displaces a waveform cell on the middle row.
- Playback-mode fixed-time jumps keep their own seek routine (it never refuses a clamped backward jump, unlike beat jumps); only the beats table is shared.
- Visual check: the 1-beat ghost dropped (too close to aim); detail ghosts repositioned as a column offset from the playhead's centre column — rounding their absolute sample through the buffer mapping separately from the playhead's rounding made labels flip a column at wide zoom. Ghosts are relative to the playhead, not the grid, so the shared-mapping rule is not what they need. 0.29.15.
- Ghosts put behind a global toggle, `ghosts_toggle` = `!`, off by default; not persisted. 0.29.16.

## Conclusion

Shipped as an experiment behind a toggle (`!`, off by default) rather than always-on as the Intent pictured — the marks read as clutter more than aim once seen, and the toggle lets the idea sit until the Clip-mode displays give it a second look. Patch bump confirmed (0.29.16). Map catch-up pending: Overview Waveform and Detail Waveform (ghost labels and their toggle), Beat Jump (single shared jump-size table; ghosts placed as a playhead offset, not via the grid mapping).
