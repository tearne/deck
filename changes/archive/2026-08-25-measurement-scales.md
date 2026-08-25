# Measurement Scales

**Mode:** Formal

## Intent

Scrolling elements have repeatedly wobbled or misaligned relative to each other because sample-to-column conversions are written ad hoc, each with its own rounding policy. Introduce concrete types for the measurement scales in play — samples, seconds, buffer columns, half-columns (dots), screen columns — and a central mapping through which all conversions happen. Column types should be constructible only via the mapping, so the compiler forces every rendered element through one shared conversion and one rounding policy; rounding intent becomes named methods rather than inline arithmetic.

First migration and proof case: the cue mark. Diagnosis (2026-08-25): the stored cue sample is exact, but `compute_cue_buf_col` floors to whole buffer columns while the viewport/playhead rounds to the nearest half-column with a parity shift — so a cue set from pause renders up to one column left of the playhead, never right. The map's Detail Waveform callout ("one shared sample-to-column mapping", three rounds of fixes behind it) is the convention this change turns into a compile-time guarantee.

*(Morphed from cue-set-accuracy, whose doubt the diagnosis confirmed.)*

## Survey (2026-08-25)

Position→column mappings concentrate in two spaces: buffer/detail (anchor + samples-per-column) and overview (track-fraction × width). The canonical policy is the detail viewport's round-to-nearest-half-column with parity. Divergent sites, ranked:

| # | Site | Divergence |
|---|------|-----------|
| 1 | cue buffer column | whole-column floor, no parity |
| 2 | post-seek drift snap | column width recomputed untruncated — snap grid ≠ buffer grid |
| 3 | detached markers on overview | truncate where overview rounds |
| 4 | detail ghost labels | whole-column round, no parity (deliberate anti-wobble) |
| 5 | click-to-seek inverse | maps column left edge; forward map rounds centres |
| 6 | paused nudge step | derived from untruncated column width |
| 7 | tick/bar grid origins | floor and ceil, defensible but unnamed |

Pure unit conversions (samples↔secs↔ms↔beats) are numerous and duplicated but internally consistent; the audio thread exchanges raw integers via atomics.

## Approach

### A new `scales` module owns units and mappings

Unit newtypes (`Samples` for f64 positions, `Secs`, `Ms`) and the mapping types live in one domain module. All rounding policy lives here and nowhere else.

### Column types are constructible only by mappings

`DetailCol` (with its dot-parity flag) and `OverviewCol` have private constructors. Ad-hoc arithmetic cannot produce a column, so the compiler enforces the map's one-mapping rule.

### Two mapping types, matching the two spaces

`DetailMap` (anchor, samples-per-column) serves the waveform, ticks, cue, ghosts, playhead and snaps, on both the render loop and the rasterisation thread — one implementation, two callers. `OverviewMap` (width, duration) serves the overview marks and owns the click inverse, mapping column centres both ways so needle drop loses its half-column bias.

### The buffer's column width is the only column width

`DetailMap` carries the master samples-per-column (the buffer's truncated integer). The untruncated re-derivations (drift snap, nudge step, `grid_snap`) read it instead — divergences 2 and 6 die by construction.

### Rounding intent is a named method

Nearest-dot-with-parity is the default. Deliberate exceptions survive as named methods (`grid_origin_floor`/`ceil` for tick origins) so a policy choice is visible and greppable, never inline arithmetic.

### One pipeline, two anchoring modes

A mark is content-anchored or playhead-anchored, chosen by what its position means, not by taste. Content-anchored marks (cue, ticks, the waveform) map their absolute sample directly. Playhead-anchored marks (ghost landings, whose sample is recomputed each frame as playhead + delta) map the anchor through the identical pipeline and add their once-rounded delta in the map's own dot units — rounding doesn't commute with subtraction, so absolute-mapping a moving sample oscillates at boundary crossings. Both modes are `DetailMap` methods built from the same primitives; character quantisation goes through the same named dot→column policy.

### Metronome beat-index conversion deduplicated

The samples→ns→beat-index conversion, triplicated verbatim, becomes one function in `scales` — consistent today, and now guaranteed to stay so.

### Cue adopts the default policy

Divergence 1 is the proof case: with cue and playhead mapped by the same `DetailMap`, equal samples produce equal columns, always.

### Unit newtypes adopted where mappings go, not everywhere

The ~100 pure samples↔secs conversions are consistent and low-risk; sweeping them all is churn without a wobble payoff. They migrate opportunistically as touched code passes through the mappings. The audio thread keeps raw integers at the atomics boundary.

### Tests assert cross-element agreement

Unit tests on the mappings: forward/inverse round-trips, cue == playhead column for equal samples, snap grid == buffer grid. The class of bug gets a regression net, not just this instance.


## Plan

- [x] `scales` module: unit newtypes (`Samples`, `Secs`, `Ms`) and their conversions
- [x] `DetailMap`: anchor + master column width, absolute dot-parity mapping, named grid-origin policies, private column types
- [x] `DetailMap` relative mapping (playhead-anchored, once-rounded delta in dot units)
- [x] `OverviewMap`: centre-rounding forward map, matching inverse, private column type
- [x] Mapping tests: round-trips, cue == playhead at equal samples, snap grid == buffer grid, relative-mapping scroll stability
- [x] Rasterisation thread adopts `DetailMap` (column width, anchor, ticks, cue) — divergence 1
- [x] Render loop adopts `DetailMap` (viewport, `sample_screen_col`, `grid_snap`, drift snap, nudge step) — divergences 2 and 6
- [x] Ghost labels adopt the relative method — divergence 4
- [x] Overview marks adopt `OverviewMap` (playhead, cue, ghosts, bar ticks, detached markers) — divergence 3
- [x] Click-to-seek adopts the `OverviewMap` inverse — divergence 5
- [x] Tick/bar grid origins through the named floor/ceil policies — divergence 7
- [x] Metronome beat-index conversion deduplicated into `scales`
- [x] `centre_col` formula deduplicated (five verbatim copies)
- [x] Paused deck snaps and nudges by whole columns (scope addition at hand-back)

## Log

- Scope addition at hand-back (user-directed): the paused deck joins the character grid — whole-column marks (cue, anchor) can't render to dot precision, so a dot-resolution rest position read as wobble. Paused snap is whole-column and a nudge press moves one column of audio (`column_step_secs`, formerly `dot_step_secs` at half the size). Grid-mode cursor steps unchanged.

- The cue left the buffer pipeline entirely: the baked column, its three atomics, `store_cue` and the deck-swap arm are gone — the render loop maps `deck.cue_sample` live through the buffer's `DetailMap`. Side effect: setting a cue no longer triggers a waveform buffer rebuild.
- Survey divergence 5 (click-to-seek inverse) was a false positive: under the rounding forward map, `duration × col / width` is already the preimage centre. The adjoint test proves it; adoption changed no behaviour.
- Bar ticks: a bar landing exactly at the track's right edge now clamps to the last overview column instead of being dropped (`col_of_frac` clamps where the old code filtered). Also feeds the beat-mode click lookup at that edge.
- `service_deck_frame` takes zoom + column count instead of pre-divided `col_secs`, so the drift snap can build the buffer's truncated grid.
- Unit newtypes landed at three natural sites (cue phase ms, ghost landing samples, tap seconds); the rest of the ~100 pure conversions stay as they are, per the Approach.

## Conclusion

Shipped as 0.33.1 (minor bump confirmed; the patch covered the paused-grid scope addition). The survey inventory in this document is the fullest record of the conversion landscape and stays with the archive. Map catch-up to follow: the Detail Waveform callout (the one-mapping rule is now compiler-enforced), and any node describing paused motion in dots.
