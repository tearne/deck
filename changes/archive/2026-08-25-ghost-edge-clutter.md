# Ghost Edge Clutter

**Mode:** Wander

## Intent

Paused ghost jump markers pile up near the track edges: paused jumps clamp to 0/track-end (playing jumps are refused instead), so several jump sizes land ghosts on nearly the same columns. Fix by not rendering clamped landings — a ghost draws only where the jump lands cleanly; the track edge is its own marker. Executed jumps still clamp; only the markers change, and the ghost's meaning becomes uniform across play states.

## Conclusion

One condition in the ghost-landing pass: a landing whose unclamped target falls outside the track draws no ghost. `jump_landing` untouched — it serves execution, which still clamps. Options considered and rejected: rendering only the shortest clamped jump (more logic, still overlaps), unifying paused/playing jump rules (paused clamping is good cueing behaviour). Shipped as 0.33.2.
