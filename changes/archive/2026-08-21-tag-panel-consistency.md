# Tag Panel Consistency

**Mode:** Formal

## Intent

In the browser's compliance mode, hovering a flagged track shows the plain metadata preview; pressing `e` swaps the RHS to the differently-shaped tag editor — the panel changes form at the moment of focus, which reads as a jump. And the editor's foreground colours are muted relative to the browser/playlist family. One form, one palette.

## Approach

- One metadata panel form for preview and edit: fields plus filename section, identical layout. Preview is passive (no caret, no hints, faded by the symmetric dim); `e` brightens the same panel and adds caret and hints. The RHS never changes shape.
- Preview shows the proposed filename whenever it differs from the current — the compliance fix, visible before `e`.
- Colours join the browser/playlist family (labels 90,110,150; values 200,220,255; yellow accents) over the retained navy-and-blue frame.

## Plan

- [x] Unified renderer with optional edit state
- [x] Preview path wired through it, proposed-name-on-differ
- [x] Colour family applied

## Log

- The old `render_track_meta` retired; `Preview::Track` now carries current and proposed names so the panel preview is the editor's form minus caret and hints.

- Hand-back additions (three rounds): a rename toggle as the editor's last focus stop, settling as the section header itself — `── [x] Rename File ──`, default on, Space flips it when focused. Off: the save is tags-only, the whole Proposed line dims, and the collision check is skipped.

- Field values in passive state use the panel family's `200,220,255` (was a murky `60,80,120`); labels `90,110,150`; dividers lifted to match. Active-field text stays white with the yellow caret.

## Conclusion

Completed at v0.29.8; patch bump confirmed. Hand-backs grew the change beyond its Intent: the rename toggle (three placement rounds, settling as the section header). Map updated at wrap-up: Context Panel's preview description and Metadata Editor's fields/toggle sentences.
