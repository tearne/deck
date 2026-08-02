# Alt Modifier

**Mode:** Formal

## Intent

Make **Alt** the chord modifier the operator sees. Deck's chorded actions are currently reached and advertised by holding **Space**, but Space is a poor modifier — a repurposed character key whose held state the terminal can't reliably report, so it needs fragile stuck-key machinery. Alt arrives as a real modifier bit on each keypress: reliable, nothing to track.

Advertise chords as `alt+key` everywhere — default config, `keybindings.md`, and the in-app keyboard help — and drop Space from all of it. Space keeps working as a silent, undocumented bonus, so existing muscle memory and user configs don't break.

Also add **deck navigation**: Alt+Down / Alt+Right advance the selected deck, Alt+Up / Alt+Left retreat — mirroring pane navigation in the Zellij terminal multiplexer.


## Approach

### One chord layer, reached by Alt or Space

Rename `SpaceChord` → `Chord`. A chord fires when Alt is held **or** Space is held. Alt is the reliable path (a modifier bit on the keypress, no held-state); Space keeps its existing machinery but is no longer advertised.

### Chords are written `alt+X`; `space+X` still parses

Config expresses chords with an `alt+` prefix. `space+` continues to parse to the same `Chord`, so existing user configs keep working. A single `alt+X` binding responds to both modifiers — Space needs no separate binding.

### Special-case keys step aside for both modifiers

The nudge, cue, and BPM-ramp handlers already ignore their key while Space is held; they now ignore it while Alt is held too, so Alt and Space are fully interchangeable and neither shadows a chord.

### Deck navigation

Add `SelectNextDeck` / `SelectPrevDeck`, cycling the selected deck with wraparound. Defaults: `alt+down` / `alt+right` advance (1→2→3→1), `alt+up` / `alt+left` retreat.

### Advertise Alt, drop Space

Default `config.toml` bindings switch to `alt+`. `keybindings.md` and the in-app keyboard help relabel the chord layer from Space to Alt and drop Space entirely. The map's Keymap node (currently "three layers: plain, Shift, Space-chord") updates to name Alt — a post-build catch-up.


## Plan

- [x] Rename `SpaceChord` → `Chord`; parse both `alt+` and `space+` prefixes to it
- [x] Resolve chords on Alt-held as on Space-held, with the nudge/cue/BPM handlers stepping aside for Alt too
- [x] Add `SelectNextDeck` / `SelectPrevDeck`, cycling the selected deck with wraparound
- [x] Switch default `config.toml` chords to `alt+` and add the deck-nav arrow bindings
- [x] Advertise Alt in `keybindings.md` and the in-app keyboard help, dropping Space


## Log

- The BPM-ramp handler previously didn't check Space at all; it now steps aside for either chord modifier, keeping Alt and Space symmetric (a minor behaviour refinement — `alt+V`/`space+V` no longer ramps).
- Removed the `[SPC]` status-line indicator: it advertised a held Space, and Alt has no armed state to show (it's per-keypress).
- Alt reuses the single `Chord` bindings via the ALT modifier bit; `space+` still parses to `Chord`, so a `space+X` config and Space itself keep working, just unadvertised.
- Not interactively tested here (TUI): whether Alt+key arrives with the ALT modifier depends on the terminal — the one runtime thing to confirm.


## Conclusion

Completed, minor bump to v0.13.1. Confirmed working in the operator's terminal (Alt registers). Two behaviour refinements beyond the plan tasks, both logged: the BPM-ramp keys (`V`/`F`) no longer ramp while a chord modifier is held — so `alt+V`/`alt+F` are free to act as chords (previously they ramped regardless of Space) — and the `[SPC]` status indicator was removed.

Documentation impact — map catch-up: the **Keymap** node describes "three input layers: plain, Shift, Space-chord" and states that terminals can't reliably detect Space held. That's now the advertised-Alt layer (Space retained silently), and Alt is precisely what fixes the reliability point the node raises.
