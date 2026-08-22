# Beat Mode Needs a Grid

**Mode:** Formal

## Intent

A deck can be in Beat mode with no tempo: a never-gridded track opens in Beat by default, and the mode key enters Beat regardless. The deck then says BEAT on the readout while behaving as Playback — percentage speed, no bar markers, no ticks — a tolerated contradiction from when the mode was a global toggle. Under per-track modes the design's rule applies cleanly: Beat requires a confirmed grid. A track without one opens in Playback; the mode key refuses Beat until a tempo is set (tap, manual entry, or refinement all remain available from Playback), saying why. A record that says Beat but carries no grid loads as Playback, and the tolerance code for no-BPM Beat goes.

Observed 2026-08-21 (0.30.3) after the placeholder-BPM fix made the state visible: `"grid": null, "mode": "beat"`.

## Approach

### Beat is entered only with a recorded grid

The mode key refuses Playback → Beat while the deck has no grid, with a hint naming the way in ("Beat mode needs a BPM — tap one with B or set it with V/F", keys read from the keymap). Playback → Beat with a grid, and Beat → Playback, are unchanged.

### A track without a grid opens in Playback

The load rule becomes: the remembered mode applies only if the record has a grid; otherwise Playback. A record saying Beat with no grid — the inconsistent state — therefore opens in Playback and rewrites itself as Playback on its next save. The deck's mode before the load-time grid result arrives is Playback too, so there is never a Beat deck without a grid, even for a frame.

### Tap and manual BPM work in Playback

Today both are Beat-only, which would leave no way in. Both become mode-independent: they are pure metadata since base-bpm-pitch — `V`/`F` never alter audio, and a tap in Playback records base BPM and offset without touching `playback_speed`. This also makes the grid-refinement record true: it already describes `b`, `V`, `F` as live in any mode while detached.

### A grid coming into existence enters Beat

Whenever a deck goes from no grid to a grid — tap lock, `V`/`F` from the placeholder, the two-anchor lock — it switches to Beat, speed preserved, exactly as the mode key would. Setting a tempo is the request to use beats, and the grid appearing is the feedback; staying in Playback with an invisible grid would be a dead step. A deck that already has a grid keeps whatever mode it is in when the grid changes.

### The no-BPM tolerances go

Every `!bpm_established` allowance that made a Beat deck behave as Playback (percentage readout, no flash, speed via the Playback path, hidden ticks) is unreachable once Beat implies a grid, and is removed rather than left as dead guards.

## Plan

- [x] Extract the mode key's two conversions into named deck operations (enter Beat / enter Playback, speed preserved) and use them from the key
- [x] Mode key refuses Playback → Beat without a grid, emitting the hint with the bound tap and base-BPM key names
- [x] Load: remembered mode applies only when the record has a grid, else Playback; new decks start in Playback
- [x] Tap and base-BPM adjust (`V`/`F`) accept any mode; tap in Playback leaves `playback_speed` untouched
- [x] When a grid comes into existence (the deck's established flag flips false → true: the 8th tap on either tap path, or `V`/`F` on the placeholder), a Playback deck enters Beat
- [x] Remove the "Beat but no BPM, so behave as Playback" branches, now unreachable: readout (percentage instead of BPM, no beat flash, hidden offset segment), BPM ±0.1 keys adjusting `playback_speed`, ticks and bar markers hidden as "analysing"
- [x] Hand back: gridless track opens Playback; `` ` `` refuses with hint; tap from Playback lands in Beat with grid; `V` from placeholder lands in Beat; gridded track still opens in its remembered mode; a `mode: beat, grid: null` record heals

## Log

- The Intent's "two-anchor lock" does not exist in code — the grid-anchors build ended on the single free-cursor anchor — so grid creation has exactly three sites: the two tap-lock paths and `V`/`F`. Both tap paths now share one operation that reads the audible speed in the mode's own terms, since in Playback the BPM field is stale.
- The mode-key refusal shows as a 5 s hint naming the bound tap and base-BPM keys.
- `V`/`F` in Playback with a grid already present updates the tempo without touching audio or mode; the renderer's ratio stays percentage-based.
- 0.31.0 for hand-back.
- Hand-back: the `Nbr` bar-interval readout showed in Playback though the markers it describes are suppressed there; now Beat-only. 0.31.1.
- Hand-back: Playback-mode jumps use the track's real beat period when a grid is known (still plain time offsets, no phase), 0.5 s only without one. Applies to the detached cursor too. 0.31.2.

## Conclusion

Shipped at 0.31.2 (minor bump confirmed). Two hand-back additions beyond the Plan, both "Playback hides grid machinery" consequences: the `Nbr` bar-interval readout is Beat-only, and Playback jumps use the track's real beat period when a grid is known. Map catch-up pending: Mode (Beat needs a grid; a gridless track opens Playback; a grid appearing enters Beat; mode key refusal hint), Beat Grid (tap and `V`/`F` in any mode), Beat Jump (Playback unit is the known beat, else 0.5 s), Overview Waveform (`Nbr` Beat-only). The root node's "deliberately excludes…" line remains deferred.
