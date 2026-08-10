# Resolution Complexity Review

**Mode:** Explore

## Intent

Track resolution has accumulated mechanisms — hint plus hash confirm, a library search with duration and file-size pre-filters, a descriptive fallback with similarity ranking, tags refresh — and planning nearly added three more ([[playlist-method-migration]]) for a version bump that has never happened.

Before building anything further here, establish whether the design is doing more than the problem requires, and whether any of it is a wrong turn worth undoing while there are no playlists in the wild to migrate.

## Approach

### Code is the source of truth

The review reads the implementation and treats map prose as a claim to check against it, not as the description of record. Planning errors in this area all came from the reverse.

### Scope is the path from entry to file

In: the resolution steps, their pre-filters, the descriptive fallback, and the write-backs performed on success — hint rewrites and tags refresh — which are where resolution can quietly lose information. Out: payload extraction internals, pinned by the corpus and verified as a separate conformance contract, plus resilient writes, backups, and editor UX.

Both Deck's implementation and the portable spec are in scope. A review that cannot question the spec cannot find a wrong turn that originated there.

### A mechanism earns its place by naming a reachable failure

Each is judged against one test: does it prevent a concrete failure that is still reachable given the other mechanisms, and not already covered more cheaply? Removing or narrowing a mechanism is a permitted conclusion, while there are no users or playlists in the wild.

### Findings only; changes spin off

The output is a per-mechanism judgement recorded in this document. Any resulting code or spec edit becomes its own change, so the review cannot quietly become a rewrite.

## Plan

**Topics**

- Each resolution step judged against the reachable-failure test, verdict recorded per step.

- The duration and file-size pre-filters: whether both earn their place, and whether their thresholds match what they are calibrated for.

- The descriptive fallback: whether similarity ranking plus operator confirmation is the right recovery, or more than the problem needs.

- The success write-backs: whether hint rewrites and tags refresh can lose information, and whether any guard is warranted.

- The unreproducible-method guard: whether refusing to hash and degrading to the fallback suffices on its own, without migration machinery.

- Identity, description and hints: whether each still earns its place in resolution.

- Divergences between the spec and Deck's implementation, recorded as found.

**Done when** every mechanism in scope carries a verdict — keep, narrow, or remove — each naming the reachable failure it does or does not prevent, and each proposed change is written up as its own change document.

## Findings

### The pre-filters trade correctness for speed, and can only lose

The hash confirm is authoritative. Both pre-filters exist solely to avoid hashing, so they can never produce a false positive — only false negatives. A candidate they wrongly exclude is a track whose hash *would* have matched exactly, dropped to the descriptive fallback and made to demand operator confirmation.

That reframes them: they are not safety mechanisms, they are an optimisation that costs correctness. Both must therefore justify their exclusions, not just their savings.

**Verdict: narrow.** Screen on size alone, and hash-confirm anything that passes.

### Cheapest-first is claimed but not implemented

`Library::cheap_probe` returns duration and size together, so every candidate pays a file open plus container parse (`probe_duration_secs`, `src/audio/mod.rs:684`) before either test runs. `within_tolerance` then tests duration first. The genuinely cheap test — a `stat` for size — never gates the expensive one.

The spec's step 2 says "Screen candidates cheapest-first … (no decode)". The ordering half of that claim is not honoured.

**Verdict: divergence, fix in implementation.** Splitting the probe so size gates duration is the minimum; removing the duration screen entirely follows from the finding above.

### Resolution cost is quadratic in a way nothing bounds

Each `resolve` that misses at step 1 calls `Library::candidates()` — a full recursive walk of the library root — then probes every file. Step 3 walks and probes the whole library *again*, adding a tag read per candidate. Nothing caches between steps or between entries, so `recompute_status` (`src/main.rs:284`) costs one or two full library scans **per unresolved entry**.

A playlist with a handful of missing tracks rescans the entire library once or twice for each of them, on every status recompute.

**Verdict: keep the steps, fix the shape.** The library scan belongs outside the per-entry loop.

### Heal and recompute do the same work twice

`heal_playlist` (`src/main.rs:528`) resolves every entry, and `recompute_status` immediately resolves them all again — both are invoked on the same panel playlist at `src/main.rs:2151–2153`. `heal_playlist` discards everything except relocated hints; `recompute_status` recomputes the identical resolution to derive status.

**Verdict: merge.** One traversal yields both the healing and the status.

### Tags refresh cannot tell "changed" from "unreadable"

`changed_description` (`src/playlist/mod.rs:216`) accepts any difference, so an all-empty read replaces a good description. The stored description is the only record of a track once its file is missing, and the input the fallback matches on — so the failure mode is losing the recovery data at the moment recovery is most needed.

Judging tag *quality* is out of the question, but empty-vs-populated is not a quality judgement.

**Verdict: narrow.** Decline a refresh that empties a populated description.

### Viewing a playlist can rotate its backups

`recompute_status` persists whenever resolution heals anything (`src/main.rs:294`), and every write rotates `.bak1→.bak2→.bak3` (`src/playlist/mod.rs`, `rotate_backups`). A single healed hint on open therefore discards the oldest backup. Three opens retire every backup a corruption would have needed.

**Verdict: narrow.** Backup rotation should track operator edits, not incidental healing.

### The fallback gives up when tags changed with the audio

`descriptive_candidates` requires `score > 0`, and `description_similarity` counts exact case-insensitive field equality. A re-encode that also retagged — the common "cleaned up my library" case — scores zero and yields `Unavailable`, with no candidates offered at all, despite duration being a strong signal on its own.

**Verdict: narrow.** Duration proximity alone should be enough to offer a candidate.

### Steps 1, 3 and 4, and the method guard

Step 1 prevents loading a different file that took over a remembered path — reachable through ordinary path reuse, and costs one hash. Step 3 is the only recovery for a re-encode, where no hash can ever match. Step 4 prevents silent loss. The unreproducible-method guard degrades to step 3 and asks, which is correct without any migration machinery — as concluded in [[playlist-method-migration]].

**Verdict: keep, all four.**

### Identity, description and hints

Identity and hints earn their place plainly. Description carries two jobs — display name and fallback matching key — and it is that second job, not the first, that makes the unconditional refresh above a data-loss risk rather than a cosmetic one.

**Verdict: keep, with the refresh narrowed.**

### The 1% size rationale does not match its stated purpose

`resilient-playlists/map.md:228` justifies 1% as accommodating "minor container rewrites that don't change the audio payload". Embedded artwork is exactly such a rewrite and routinely exceeds 1%. The threshold does not deliver what its own rationale claims.

Narrowed by the first finding — if size is the only screen, its tolerance matters more, not less.

**Verdict: spec and implementation both need a number that matches the intent.**

## Conclusion

The review found no mechanism worth removing — every verdict was keep or narrow. The complexity that prompted it lives in the wiring between mechanisms, not their count, which is why it resisted per-mechanism inspection.

Five proposals spun off; no code or spec was touched, as the Approach required. The two cost findings are read from call structure rather than profiling, and should be measured before that work is sized.

## Log

- The review's sharpest finding was not a mechanism being unnecessary but a framing error: the pre-filters read as safety checks and are in fact a speed optimisation that can only lose tracks, never protect them. Every other pre-filter question follows from that.
- Cost findings (per-entry library scans, duplicated heal/recompute traversals) were not anticipated by the Topics, which were framed around whether mechanisms earn their place. They surfaced from reading call sites rather than the engine.
- No mechanism was found to be genuinely unnecessary. The complexity concern that prompted the review is real, but it lives in how the mechanisms are wired together, not in their number.
