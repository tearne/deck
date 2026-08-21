# Beat Mode Needs a Grid

**Mode:** Formal

## Intent

A deck can be in Beat mode with no tempo: a never-gridded track opens in Beat by default, and the mode key enters Beat regardless. The deck then says BEAT on the readout while behaving as Playback — percentage speed, no bar markers, no ticks — a tolerated contradiction from when the mode was a global toggle. Under per-track modes the design's rule applies cleanly: Beat requires a confirmed grid. A track without one opens in Playback; the mode key refuses Beat until a tempo is set (tap, manual entry, or refinement all remain available from Playback), saying why. A record that says Beat but carries no grid loads as Playback, and the tolerance code for no-BPM Beat goes.

Observed 2026-08-21 (0.30.3) after the placeholder-BPM fix made the state visible: `"grid": null, "mode": "beat"`.
