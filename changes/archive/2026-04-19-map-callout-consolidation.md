# Map Callout Consolidation

## Intent

The upstream `changes/agents/` process in the cod project has been revised to collapse all map callout kinds (`[!DECISION]`, `[!ASSUMPTION]`, `[!CONSTRAINT]`, `[!TODO]`) into a single `[!IMPORTANT]` callout, with tighter guidance on when to use it. Two follow-ups are needed in this project:

1. **Refresh `changes/agents/`** in this project from the upstream cod version. Several other changes are included in that upstream update (approval wording, Feedback/Conclusion templates, post-feedback return path, Approach queue clarification, `active.md` format, README engagement caveat) — pulling them in keeps the process docs aligned.

2. **Audit the 5 existing callouts in `map.md`** against the new stricter bar. Reserve `[!IMPORTANT]` for load-bearing points where skimming would lose the reader — design trade-offs, non-obvious assumptions, constraints that shape the whole node. Not for every notable fact.

Current callouts (for audit reference):

- Line 72 — `[!DECISION]`: "Three deck instances share one conceptual model."
- Line 132 — `[!DECISION]`: pre-render vs per-frame waveform, avoids wiggle
- Line 163 — `[!ASSUMPTION]`: char colour on half-col shift
- Line 201 — `[!DECISION]`: session persistence (mode cache restore)
- Line 347 — `[!DECISION]`: never block the audio thread (duplicates Application-level constraint at line 17)

The audit should: rewrite the label where kept, fold into prose where the callout isn't earning its keep, and remove outright where duplicated elsewhere.

## Approach

**Scope.** One change covering both the agent-docs refresh and the callout audit. Phased execution — refresh first, audit second.

**Agent-docs refresh.** Already completed by the user; `/root/deck/agent/` files verified identical to `/root/cod/agent/` equivalents. No further action needed on that phase. `MAP-GUIDANCE.md` carries the new stricter callout guidance, which is the basis for the audit below.

**Callout audit decisions.** Each of the five existing callouts in `map.md` has been reviewed against the new bar:

1. **Line 74 (Deck)** — `[!DECISION]` "Three deck instances share one conceptual model…" — **fold into prose**. Meta-guidance about reading the map, not a design trade-off.
2. **Line 134 (Wide Buffer)** — `[!DECISION]` "Pre-render rather than render each frame…" — **keep, relabel `[!IMPORTANT]`**, wording unchanged. Load-bearing design trade-off that justifies the node.
3. **Line 205 (Sub-Column Smoothing)** — `[!ASSUMPTION]` "Char colour on half-col shift…" — **keep, relabel `[!IMPORTANT]`**, wording unchanged. Non-obvious assumption that makes the smoothing trick viable.
4. **Line 243 (Beat vs Vinyl Mode)** — `[!DECISION]` "Session persistence…" — **fold into prose**. Secondary detail, not load-bearing.
5. **Line 389 (Audio Pipeline)** — `[!DECISION]` "Never block the audio thread…" — **remove outright**. Duplicates the Application-level constraint at line 17.

**Review cadence.** Single end-of-edits review. No `Cargo.toml` bump — doc-only change.

## Plan

- [x] UPDATE map.md: fold the Deck node callout into the preceding prose
- [x] UPDATE map.md: relabel the Wide Buffer node callout as `[!IMPORTANT]`
- [x] UPDATE map.md: relabel the Sub-Column Smoothing node callout as `[!IMPORTANT]`
- [x] UPDATE map.md: fold the Beat vs Vinyl Mode node callout into the preceding prose
- [x] REMOVE from map.md: the Audio Pipeline node callout (duplicates the Application-level constraint)
- [x] REVIEW: end-of-edits walk-through with user

## Conclusion

Completed.
