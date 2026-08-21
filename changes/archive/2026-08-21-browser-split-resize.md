# Browser Split Resize

**Mode:** Formal

## Intent

*(Parked — captured as an aside.)*

The browser is a fixed ~70/30 split (file list / context panel). Let the operator resize it — **Alt+Left / Alt+Right** to widen/narrow the panel — persisting the chosen ratio.

Design note: `alt+left` / `alt+right` are currently bound to `select_prev_deck` / `select_next_deck` (alongside `alt+up` / `alt+down`). Resizing with those keys collides with deck cycling — so this needs a rebind decision (deck cycling to up/down only, or the resize onto different keys).

## Approach

The panel ratio becomes session state (clamped 15–70%, 5% steps), consumed by both split sites (browser screen, offer-editor fallback). Two configurable actions on the Alt layer, handled only while the browser is open. Full hjkl≍arrows consistency: `alt+h`/`alt+left` widen, `alt+l`/`alt+right` narrow, and deck select gains `alt+up`/`alt+down` beside `alt+j`/`alt+k`. The original Intent's key collision dissolved when the keymap rework freed the arrow chords.

## Plan

- [x] Ratio in session state, clamped and stepped, persisted
- [x] `panel_widen`/`panel_narrow` actions wired, browser-open only
- [x] Deck select gains `alt+up`/`alt+down`; keybindings.md updated

## Conclusion

Completed at v0.29.0; minor bump confirmed. One map touch at wrap-up: Session State enumerates what is remembered, so the panel width joins its list.
