# Loop Bounds Atomic Race

**Mode:** Formal

## Intent

The background detail renderer reads `loop_active`, `loop_start`, and `loop_end` as three separate `Relaxed` atomic loads at `src/render/mod.rs:296–298`. When a trim handler stores new bounds mid-render, the renderer can sample a mismatched pair — old start with new end, or vice versa — and briefly compute inverted bounds. Bounds validation downstream then disables wrapping for that frame, producing a one-frame flicker on rapid trim presses. Tighten the load order so the renderer always sees a consistent triple.

## Approach

Pack the two bounds into one `AtomicU64` (start and end as `u32` halves — a 27-hour ceiling in mono frames at 44.1 kHz). The writer stores bounds before the active flag, so a reader seeing `active` always finds real bounds; the reader unpacks a single load, making a mismatched pair impossible by construction. The audio thread's loop atomics are untouched — trims change one bound per press there, so its pairs are already consistent. The `end > start` check stays as belt-and-braces.

## Plan

- [x] Pack renderer loop bounds into a single atomic (writer order, reader unpack)

## Log

- The active flag's load/store pair was upgraded Relaxed → Acquire/Release so the bounds-before-flag ordering actually holds across threads; the packed bounds themselves stay Relaxed.

## Conclusion

Completed at v0.22.2; patch bump confirmed. No map impact.
