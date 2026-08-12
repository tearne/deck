# Deck Row Condense

**Mode:** Formal

*(Spun out during message-stream planning; message-stream has since landed.)*

## Intent

Since the message stream routed all events to the global bar, each deck's title row carries only the track title, playlist badge, `[BPM][Tick][Cue]` indicators, and the deck-attached prompts (BPM confirmation, rename offer). Review whether the row still earns its screen line — the suspicion is things can be condensed, e.g. by removing the BPM/Tick/Cue indicators.

## Approach

### What the row carries

Track title and playlist badge (shown nowhere else), the three indicators (redundant — below), and the deck prompts (relocatable — below).

### The indicators are redundant

- `[BPM]` — the info row's readout already signals it: a BPM number appears only when established, a percentage otherwise, and only an established BPM beat-flashes.
- `[Cue]` — a set cue draws its line on the overview, which always shows the whole track.
- `[Tick]` — lights whenever a cue is set (the common case). The cue mark alone is sufficient; the rare "offset placed without a cue" state goes unsignalled, accepted.

### Prompts override the info row

The BPM confirmation (auto-rejects at 15 s) and the rename offer's active phase (10 s) temporarily replace the info row's content — they are momentary, and the readouts return when they resolve. The rename offer's lingering phase (amber `⚠`, the browser's non-compliance marker) sits at the right end of the overview's top row; the bar-interval legend yields while it lingers. Treatment to be reviewed at hand-back.

### The row goes

With indicators gone and prompts relocated, title + badge overlay the overview waveform's top-left — permanent, so the track name stays glanceable — mirroring the bar-interval legend already overlaid top-right. The overlaid text keeps the old row's navy background, so each deck keeps its distinct title colouring. The row is deleted, freeing three screen lines; an empty deck shows a dim "no track" in the same spot.

### Map

Affects the Deck area nodes (Deck, Renaming, Overview Waveform) and the Messages surfaces sentence. Catch-up after build, per MAP-GUIDANCE.

## Plan

- [x] Title + badge overlaid on the overview top-left, navy text background; dim "no track" when the deck is empty
- [x] Delete the per-deck title rows from the layout; selected-deck accent strip bounds follow
- [x] BPM confirmation and rename offer (active phase) override the info row's content
- [x] Lingering rename offer at the overview's top-right; bar-interval legend yields while it lingers
- [x] Remove the `[BPM][Tick][Cue]` indicators and their renderer

## Log

- The title row also carried the deck number (yellow when selected) and the loading-progress label; both moved into the overview overlay.

- The legend "yields" by z-order alone: the lingering offer renders after the overview, covering the top-right corner where the legend sits — no legend code changed.

- On terminals short enough that an overview falls off the bottom entirely, its deck's title disappears with it (the old row survived slightly longer in the compression order). Accepted.

- The layout spacer index moved (c[16] → c[13]); browser, album art, help, and history all render in that spacer.

## Conclusion

Completed at v0.19.1; minor bump confirmed. The review verdicts (lingering-offer placement, overlay feel at minimum height) roll into the follow-on [overview-corners](2026-08-12-overview-corners.md) change, which reworks this surface further — map catch-up for the deck area waits for that change rather than describing a layout about to move again.
