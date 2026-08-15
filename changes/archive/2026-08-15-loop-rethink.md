# Loop Rethink

**Mode:** Explore

## Intent

Looping deserves a proper design rather than more growth from the deck-3 prototype. This change's work is the thinking, not code: capture the big picture — what looping in deck should be, the new functionality, how it organises against existing features, and what it means for the keymap. The application's stated rationale ("deliberately excludes loops…") no longer reflects intent and needs revising along the way. The deliverable is an approved design plus filed Intents for a sequence of follow-on implementation changes; code belongs to those, not here.

## Approach

### The design lives here

A **Design** section grows in this document through Build; archived whole, it stays the record. Map nodes follow implementation, per the sync rule — with one exception to settle during the work: when the root node's "deliberately excludes loops" line stops describing intent.

### Biggest questions first

Topics ordered so early answers constrain later ones: vision before functionality before organisation before keys before sequencing. Each settles by drafted text and your adjudication, map-negotiation style.

### The prototype is evidence, not a base

Its lessons — tap capture, trim-by-ear, the panel view — inform the design; nothing is owed to its implementation.

## Plan

**Topics**

- Vision and rationale — what looping is *for* within deck's ethos; the exclusion line's revision

- Functionality — capture, refinement, exit, persistence, the beat-grid relationship, which decks

- Feature organisation — how loop mode sits against transport, cue, and beat jump; what of the prototype survives

- Keymap — keys and layers for all of it, across three decks

- Sequencing — slice the build into follow-on changes; draft their Intents

**Done when** the Design section is approved and the follow-on Intents are filed in `changes/open/`.

## Design

### Vision

Deck inhabits the space between a pair of turntables and a DAW: the performance elements of vinyl mixing — beat-matching by ear, transitions held by hand — combined with the track-manipulation vocabulary of a DAW: looping, jumping between sections, layering across decks, and real time exploration.

### Three deck modes

Each deck runs in one of three modes:

- **Playback** — today's vinyl mode: no BPM required; speed and pitch change; jumps at fixed time intervals.

- **Beat** — today's beat mode, refined. Jumping by beats/bars requires a correct BPM; grid *alignment* stays optional (it doesn't affect jump distances). The tempo workflow sharpens: an **anchor marker** near the start and another near the end of the material, then a few taps lock in a BPM partially constrained by the two markers. Grid prep must be possible while the track plays in Playback mode, without interrupting playback. New **ghost playheads** on the overview show where each beat-jump key would land (e.g. ±8 bars).

- **Clip** — new. The track is sliced into start points perfectly aligned to the beat grid, each paired with an end slice point (not necessarily the adjacent one). Clips loop, run on into the next, or are jumped between with phase-preserving arithmetic — same-length clips land at the same position, longer destinations keep the offset from clip start, shorter ones take position mod destination length (exact rules to be settled by prototyping). BPM and jump controls work as usual; the beat-jump keys may be repurposed for clip-to-clip jumps in this mode. Future: defined clip sequences.

Modes are **per deck** — a deck looping clips over another free-running in Playback is the point. The global vinyl/beat toggle retires; the mode key cycles Playback → Beat → Clip on the selected deck, and a loaded track opens in whichever mode it last used (per-track memory).

### Feature allocation across modes

- All modes: speed, pitch, nudge, needle drop.
- Beat and Clip: metronome, BPM adjust, tap, detection.
- Jumps: fixed-time (Playback), beats/bars (Beat), clip-to-clip (Clip).
- **Cue** exists in Playback and Beat only — clip mode has no cue; a clip start is the sharper concept.
- **Grid datum**: the two anchors when defined; else the cue; else track start.

### Displays

Slices mark both overview and detail, in the manner of the beat-marker triangles. Ghost playheads stay subtle on the overview. Clip mode adds overlays: the active clip, whether it loops, what comes next. The prototype's three-panel loop view dies with the prototype.

The loop prototype is done and gets removed. Its one carried lesson: a tiny crossfade makes loop seams smooth — clip boundaries (and possibly beat-mode jumps) get the same treatment, potentially configurable.

### Persistence

Database per identity if needed, but first explore embedding grid and clips in the track's own metadata — it travels with the file, and tag writes are identity-safe by construction (identity excludes tags by design).

### Playlists as the record box

A playlist is the box of records planned for the set, not yet committed to a deck: an easy selector that loads any of its tracks onto any deck (the context panel half-does this today). Sequential auto-play through a given deck — today's attach-and-advance — becomes an *option*, available in Playback and Beat modes; a clip-looping deck never auto-advances regardless. Run-on past the last slice reaches the real track end and behaves like any end of track. The end-of-track warning flash is suppressed in Clip mode.

### Keymap direction

- **Space becomes the reset modifier** over the mixer keys (the resets today buried in Alt chords); the Alt layer over those keys is reclaimed for features — deck switching now, clip controls later.
- **`Alt+j` / `Alt+k`** cycle decks Zellij-style; the existing selectors (`Alt+1/2/3`, `Alt+arrows`) are dropped, freeing those chords.
- **`` ` ``** stays the mode key, now cycling the selected deck's three modes.
- Clip mode's key surface is designed in its own changes, drawing on the freed Alt+mixer chords.

### The change sequence

Filed as numbered proposals in `changes/open/` — tens spacing so insertions slot in without renumbering:

1. `10-keymap-rework` — Space as reset modifier, Alt+mixer layer freed, `Alt+j`/`Alt+k` deck cycling, old selectors dropped. First, per the deck-switching priority.
2. `20-per-deck-modes` — the global toggle becomes a per-deck Playback/Beat cycle (Clip joins when it exists); per-track last-used-mode memory; the loop prototype removed.
3. `30-grid-anchors` — the two anchor markers, taps constrained by them, grid prep during uninterrupted playback.
4. `40-ghost-playheads` — subtle overview marks at each jump key's landing point. Small; could float.
5. `50-clip-mode-core` — slices, start→end pairing, loop/run-on/jump-between, displays, crossfade; phase-jump rules prototyped inside it.
6. `60-clip-persistence` — metadata embedding explored against the database fallback.
7. `70-record-box-playlists` — any-track-any-deck selection as the primary model; deck auto-play opt-in.

Later, unsequenced: clip sequences; deck count / on-demand creation.


## Conclusion

Design approved; the seven sequenced Intents (10–70) are filed and `changes/open/` reads as the roadmap. No code and no version — this change shipped thinking. Two deferred map notes: the root node's "deliberately excludes loops" line and the vision's migration into the map wait for the features to land, per the sync rule; the Design section archived here is the reference until then.
