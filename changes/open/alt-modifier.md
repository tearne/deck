# Alt Modifier

**Mode:** Formal

## Intent

*(Under consideration — parked until wanted.)*

Add **Alt** as an alternative chord modifier alongside the existing `space+key` chords. Alt is delivered as a modifier bit on each key's Press event rather than as a repurposed character key, so holding Alt and pressing keys is reliable — it sidesteps the fragile space-held tracking (stuck-modifier repeats, missing Release events) that the space chord fights.

Also add **deck navigation with Alt+Up / Alt+Down** — cycling the selected deck, mirroring pane navigation in the Zellij terminal multiplexer.

Design point to settle when picked up: this is reliable only for Alt **combined with another key** (a chord). A standalone "Alt is held right now" state that modifies continuous, non-keypress behaviour would hit the same terminal Release-event unreliability as space, so any Alt use must be key-combination-shaped. Also weigh terminals/WMs that intercept Alt+letter.

Relates to the existing `SpaceChord` binding — an `alt+key` form would slot in beside (or replace) it.
