# Loop Bounds Atomic Race

## Intent

The background detail renderer reads `loop_active`, `loop_start`, and `loop_end` as three separate `Relaxed` atomic loads at `src/render/mod.rs:296–298`. When a trim handler stores new bounds mid-render, the renderer can sample a mismatched pair — old start with new end, or vice versa — and briefly compute inverted bounds. Bounds validation downstream then disables wrapping for that frame, producing a one-frame flicker on rapid trim presses. Tighten the load order so the renderer always sees a consistent triple.
