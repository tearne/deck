# Per-Deck Modes

**Mode:** Formal

*(Part of the loop-rethink sequence — see the design for ordering and context.)*

## Intent

The global vinyl/beat toggle becomes a per-deck mode cycled with `` ` ``: Playback (today's vinyl) and Beat, with Clip joining when it exists. A loaded track opens in the mode it last used (per-track memory). The deck-3 loop prototype is removed as part of the ground-clearing; its crossfade lesson carries forward to clip boundaries.

Design: [loop-rethink](../archive/2026-08-15-loop-rethink.md).

## Approach

- The global `vinyl_mode` bool becomes `DeckMode` (`Playback` | `Beat`; Clip later) on the deck. Everything keyed off the global — percentage vs BPM display, time vs beat jumps, BPM suppression, bar markers, the speed-preserving toggle conversion — moves per-deck. The toggle cycles the *selected* deck; an empty deck has no mode and the toggle is a no-op there.
- Vocabulary: "vinyl" → "Playback" in code and docs. The action renames to `mode_cycle`; `vinyl_mode_toggle` still parses as an alias.
- Per-track memory: a `mode` field on the track-database entry, applied at load. Metadata embedding stays with change 60.
- Session State's global mode entry retires.
- Each deck shows its mode as a `BEAT│` / `PLAY│` prefix on its readout corner; the global `[VINYL]/[BEAT]` tag dies.
- The loop prototype comes out whole — actions, keys, state, audio atomics, renderer machinery, panels, jump-key trim overrides. Git history keeps the crossfade code for clip-core.

## Plan

- [x] `DeckMode` on the deck; global bool and session entry removed; selected-deck toggle (no-op when empty) preserving audio speed
- [x] Vocabulary rename and `mode_cycle` action with alias
- [x] Per-track last-used mode in the track database, applied at load
- [x] `BEAT│`/`PLAY│` readout prefix; global tag removed
- [x] Loop prototype removed wholesale
- [x] Config defaults and keybindings.md updated

## Log

- The remembered mode applies when the track's identity arrives (a beat after load), since the memory is keyed by hash — same timing as cue/gain restore.

- The BPM-detect and metronome gates moved inside the deck borrow, so "Beat mode only" is now checked against the deck the action would touch, not a global.

- Old `session.json` files still parse — the dropped `vinyl_mode` field just goes unread. Old configs work via the `vinyl_mode_toggle` alias.

- The loop removal also took two orphaned render helpers (`render_braille_single_dot`, `peaks_for_range`) and `TrackingSource`'s pitch-flush handle, whose only trigger was the loop wrap.

- Help overlay and keybindings art now say "` mode"; the loop section is gone from the docs.

## Conclusion

Completed at v0.26.0; minor bump confirmed. Map updated at wrap-up across eight nodes — the Mode node rewritten per-deck, Loop Prototype deleted, and Beat Jump's long-standing "lands on the next tick mark" claim corrected to phase-preservation (user-caught). The base-bpm-pitch aside was filed during review. The root node's excluded-loops line stays true again until clip-mode-core makes it false.
