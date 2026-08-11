# Resolution Scan Cost

## Intent

*(Proposed by [[resolution-complexity-review]].)*

Resolving one entry that misses its hint walks the entire library root and probes every file; the descriptive fallback then walks and probes it all again, adding a tag read per candidate. Nothing is cached between steps or between entries, so recomputing status for a playlist costs one or two full library scans **per unresolved entry**.

Separately, `heal_playlist` and `recompute_status` resolve the same entries twice in a row on the same playlist — the first keeps only relocated hints, the second recomputes identical work to derive status.

Scan the library once per operation rather than once per entry, and derive healing and status from a single traversal.

## Approach

### One snapshot per operation, and nothing longer-lived

The caller probes the library once and resolves every entry against that snapshot. No cache outliving the operation, and so no invalidation or staleness rules — that machinery would need evidence to justify, and we chose not to gather any.

### The snapshot is passed in, not hidden inside the library

`resolve` takes the probed candidates as an argument rather than fetching them. A memoising wrapper would hide the cost behind the same call that has it today; an argument makes it visible in the signature and keeps the engine free of interior state.

### Duration and size only; descriptions stay lazy

Screening needs duration and size for every candidate, but descriptions are only needed by the fallback and by tag refresh on the matched file. Probing tags for the whole library up front would add cost to operations that currently never pay it.

### One traversal serves healing and status

Where both are wanted they derive from the same resolution rather than repeating it. Decks keep healing only: knowing which of a deck's tracks are unavailable is worth having, but it arrives with the change that displays it ([[deck-playlist-warning]]) rather than sitting unread here.

## Plan

- [x] Build a library snapshot of path, duration and size, probed once per operation
- [x] Resolve entries against a supplied snapshot rather than fetching candidates per entry
- [x] Derive the playlist panel's repairs and statuses from one pass
- [x] Share one snapshot across every playlist healed when a workspace is set
- [x] Cover snapshot reuse in tests — resolving many entries probes the library once

## Conclusion

The fix added no mechanism: a snapshot type, an argument, and the deletion of a redundant call. That matters given the review's finding that the complexity here was wiring rather than count.

Skipping measurement cost nothing durable — the counting test asserts the shape, one screening per operation, rather than a timing, so it holds on any machine and at any library size.

No documentation impact: the spec describes what resolution does, not how often an implementation scans.

## Log

- The panel's duplicated work turned out to need no merging code: `recompute_status` already adopts healed entries as it goes, so the preceding `heal_playlist` call was pure repetition. Deleting it and letting the recompute persist (`persist: true`) leaves one pass doing both.
- Step 2 and step 3 previously probed every candidate separately within a single `resolve`. Both now read the one snapshot, so even a single unresolved entry pays one screening rather than two.
- Six `resolve` call sites; only the workspace-set pass and the panel recompute resolve many entries, so only those share a snapshot. The four single-entry sites build their own, which costs exactly what they cost before.
- Version 0.15.24 → 0.15.25.
