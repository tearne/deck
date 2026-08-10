# Resolution Scan Cost

## Intent

*(Proposed by [[resolution-complexity-review]].)*

Resolving one entry that misses its hint walks the entire library root and probes every file; the descriptive fallback then walks and probes it all again, adding a tag read per candidate. Nothing is cached between steps or between entries, so recomputing status for a playlist costs one or two full library scans **per unresolved entry**.

Separately, `heal_playlist` and `recompute_status` resolve the same entries twice in a row on the same playlist — the first keeps only relocated hints, the second recomputes identical work to derive status.

Scan the library once per operation rather than once per entry, and derive healing and status from a single traversal.
