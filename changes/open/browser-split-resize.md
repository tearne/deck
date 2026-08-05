# Browser Split Resize

**Mode:** Formal

## Intent

*(Parked — captured as an aside.)*

The browser is a fixed ~70/30 split (file list / context panel). Let the operator resize it — **Alt+Left / Alt+Right** to widen/narrow the panel — persisting the chosen ratio.

Design note: `alt+left` / `alt+right` are currently bound to `select_prev_deck` / `select_next_deck` (alongside `alt+up` / `alt+down`). Resizing with those keys collides with deck cycling — so this needs a rebind decision (deck cycling to up/down only, or the resize onto different keys).
