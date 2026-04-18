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

Awaiting Approach discussion.
