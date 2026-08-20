# Grid Anchors

**Mode:** Formal

*(Part of the loop-rethink sequence — see the design for ordering and context.)*

## Intent

The beat-grid workflow sharpens: an anchor marker near the start of the material and another near the end, then taps constrained by the two lock in BPM and offset together. Grid prep must work while the track plays in Playback mode without interrupting playback. The anchors take over as the grid datum (falling back to cue, then track start).

Design: [loop-rethink](../archive/2026-08-15-loop-rethink.md).

## Approach

### Successive refinement, not inference

The grid is tuned by looking, not computed from measurements: pin one anchor on a downbeat, then jump progressively further out, adjusting the base BPM until the ticks sit on the transients at each distance. Each jump multiplies the leverage of the same visual alignment, so a few stages land within hundredths of a BPM. Nothing is inferred, so nothing can be silently wrong; every mis-tune is recovered by tuning while looking. (An earlier two-anchor design computed the BPM from pin timings — replaced mid-build; see Log.)

### The detached view is the whole mode

`g` toggles a **detached view** on the selected deck: playback runs on untouched while the detail waveform stops tracking the playhead and anchors to a free cursor. Jumps and nudge steer the cursor; `g` or Esc re-attaches. Everything else is normal mode — `V`/`F` tune the real base BPM live (pure metadata since base-bpm-pitch), `D`/`C` shift the phase, `b` taps a rough starting point — with the tick marks rendering the real grid as it changes, in any deck mode while detached. There is no session, no provisional state, and nothing to commit: every edit is already live and audio-safe.

### The persisted anchor

`a` (shadowing pitch-up only while detached) pins the anchor exactly at the cursor. The anchor is the grid's **phase source**: `offset_ms` derives from it at 1 ms precision, re-derived whenever the base BPM changes; `D`/`C` shift the anchor with the offset; tap and detection results keep their tempo but defer phase to the anchor. It persists in the track database, so re-entering the detached view on a known track resumes refinement rather than restarting. Datum chain: anchor, else cue, else track start.

### Markers

While detached: a blue cursor column on the overview, amber anchor columns on both views. Attached, the markers hide — the grid itself is the visible artefact.

## Plan

- [x] Detached view: `g` toggles, cursor born at the view position, jumps/nudge steer it, Esc/`g` re-attach; session machinery (taps, second anchor, lock, dashboard, swallow list) removed
- [x] Ticks render while detached regardless of deck mode
- [x] Anchor: `a` pins at cursor, persisted in the track database, offset derived at 1 ms; `D`/`C` shift it; base-BPM changes, tap, and detection re-derive phase from it; cue fallback
- [x] Arrows: `←` to anchor, `→` to the last tick
- [x] Markers: blue cursor, amber anchor, both views, detached only
- [x] keybindings.md updated to the detached-view shape

## Log

- Twelfth round: the anchor marker and view snap now go through the buffer's own arithmetic — a shared sample-to-screen-column mapping (mirroring tick extraction, including the sub-column shift) and the buffer's integer samples-per-column as the snap grid. The wiggle was f64 marker maths disagreeing with the buffer's integer division by one column at varying scroll positions.

- Eleventh round: detached ticks render as three-wide glyph markers (─┴─ / ─┬─, stem pointing at the owning deck) centred on the true tick column — the braille half-column form read off-centre against the whole-column view. Normal-mode ticks keep the braille.

- Tenth round: two-tone blue — fixed furniture (ticks, tag, anchor) darker and richer (40,100,210), the movable cursor keeping the lighter cyan (60,150,255) so it stands out against what it moves between.

- Ninth round: tick spillover characters (a tick spans two chars) now colour with their tick; the accent unified as GRID_BLUE (60,150,255), richer, and the anchor joined it — all grid furniture one colour. The detail anchor marker computes against the same whole-column view position the waveform renders at, killing the one-column wiggle while scrolling.

- Eighth round: grid-work accents unified on blue — the GRID tag and the detached deck's tick marks (per-character within the shared rows, so a row-mate deck keeps gray) match the cursor column. And the detached view snaps to whole columns: with no motion to smooth, sub-column smoothing only made waveform-to-marker registration ambiguous during anchor placement; the snap costs a one-off half-column shift at detach.

- Seventh round (rebuild to the dissolved shape): `V`/`F` and all normal keys live again while detached (only cursor-steering keys and `a`/arrows/Esc are captured); cue-set defers to an existing anchor via the shared datum function; tap and detection keep their tempo but re-derive phase from the anchor; the anchor restores from the database at load and re-derives the offset then too. Transient snap deleted. 47 tests.

- Sixth round: the transactional session was dissolved entirely — the user identified that grid edit mode had reduced to view detachment, everything else being normal-mode edits that are already live and audio-safe. Approach rewritten to successive refinement around a detached view and a persisted anchor; the two-anchor lock, taps-in-session, integer snap, and dashboard all removed.

- The session pins to the deck it opened on: selection can wander (`Alt+j/k` works mid-session), the captured keys keep acting on the session's deck, and its readout shows `GRID│` in amber.

- The lock preserves the playback speed ratio (the base-bpm-pitch rule) and emits a success event naming the locked BPM and beat count — the log records every grid lock.

- Prompt precedence: the quit confirmation still outranks the session's Esc; a displayed message loses to it (grid Esc cancels the session first, a second Esc dismisses the message).

- Hand-back correction: the first build pinned anchors at the playhead and seeked audio to navigate — wrong model. Reworked to the free cursor: playback untouched, the detail view anchors to the cursor (the wide buffer builds around it), jumps/nudge steer the cursor, and taps alone remain playhead-timed.

- While the session lives, `[`/`]` (latency), `t` (16-bar back-jump), and all jump/nudge keys are captured for the cursor — they return on commit/cancel.

- Fifth round: integer/half snap added after live testing surfaced 121.97-for-122 locks — tolerance 0.08 BPM.

- Fourth round: with both anchors pinned, the detail tick marks preview the provisional lock live (the same candidate the dashboard names), updating as taps bed in and reverting on Esc — and the preview shows even during Playback-mode prep, where committed ticks are suppressed. Overview bar markers stay on the committed grid until Enter, a deliberate before/after contrast.

- Third round: the readout's GRID tag became a live session status — `GRID AB t6 →127.98` — showing pinned anchors, tap count, and the exact BPM a commit would lock (one shared computation with the commit, so preview and result cannot disagree). Ticks still move only on Enter, by design.

- Second hand-back round: the early nudge/warp block and base-BPM ramp are gated off during a session (d/c had still been seeking audio); tempo/grid mutators (tap, bpm±, offset, cue, cue-play, mode cycle, speed reset, chorded detect) are swallowed while the session lives; anchors now mark the detail view too, mapped into the viewport around the cursor.

- Commit with fewer than four session taps falls back to the current base BPM as the `n` disambiguator, so a lock after ordinary tap-then-grid workflows needs no re-tapping.

## Conclusion

Completed at v0.27.11; minor bump confirmed. The change travelled far from its filed Intent: the two-anchor lock the loop-rethink design specified was built, tested, and replaced mid-change by the user's successive-refinement model — a detached view, one persisted anchor, and tuning by eye — which dissolved the transactional session entirely. Twelve build rounds are logged. Map updated at wrap-up (Grid Refinement node with the shared-mapping callout aimed at clip-mode-core; Beat Grid rewritten datum-first; Cue Point and Track Database touched). Three asides filed during the change: drop-detection, paused-column-snap, and earlier global-bar-position remains open. The loop-rethink design document's grid-anchors description is superseded by this change's Approach — noted here rather than editing the archive.
