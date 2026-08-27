# Default Deck Count

**Mode:** Formal

## Intent

Three empty decks by default is more than a set usually needs. Start with one deck showing; the other two are summoned on demand — `alt+N` selects deck N, summoning it if hidden — up to today's three. A visible deck can be dismissed again (rules to be settled: only when empty or stopped). Hidden decks cede their rows to the rest of the layout. Today's three-deck behaviour remains fully available; this changes the starting posture and screen economy, not capability. (2026-08-24; refined 2026-08-26. Related: [deck-selection-keys](../archive/2026-08-25-deck-selection-keys.md).)

## Approach

### Decks are a list, not slots

Between one and three decks exist, numbered contiguously — dismiss deck 2 and the old deck 3 *becomes* deck 2. Nothing is hidden-but-kept: a deck either exists on screen or is gone. The swap machinery already trades whole deck states between slots, so renumbering reuses it. Per-slot attachments move with the deck: PFL routing, an in-flight pending load, the detached grid view, and any pending load-confirm. (Palettes are per-slot in mechanism but currently identical, so reflow shows no colour change.)

### Alt+N: select, summon, dismiss

`select_deck_N` selects deck N when it exists; when N is one past the count it appends a fresh empty deck and selects it; further out it does nothing. On the already-selected deck it dismisses: an empty deck goes at once; a loaded, paused deck asks for `y` (the standing destructive-confirm pattern) since its track and state are discarded; a playing deck refuses with a notification. The last deck can't be dismissed. `alt+j`/`alt+k` cycle the existing decks.

### Selection follows the collapse

Dismissing the selected deck selects its neighbour (the deck that inherits its number, or the new last deck when the end was dismissed).

### Layout shows what exists

Absent decks contribute zero-height rows; freed rows fall to the bottom pane. Shared tick rows exist per adjacent pair of existing decks.

### Position-addressed keys act on what exists

Mixer keys for deck N do nothing when deck N doesn't exist; swaps needing an absent deck are no-ops. No remapping — the right-hand columns keep their fixed meaning.

### Deck count persists

Session State carries the count; a fresh session starts with one deck. Session restore recreates the decks its snapshots hold. A command-line track lands on deck 1.

## Plan

- [x] Deck-count state: 1–3 contiguous decks, fresh default one
- [x] `select_deck_N` dispatch: select existing / append-and-select when N is count+1 / dismiss when selected (empty at once; paused via `y` confirm; playing refused; last deck protected)
- [x] Reflow on dismissal via the swap machinery, moving PFL routing, in-flight pending loads, the detached view, and any pending load-confirm
- [x] Selection follows the collapse; `alt+j`/`alt+k` cycle existing decks only
- [x] Layout rows only for existing decks; shared tick rows per adjacent pair
- [x] Mixer and swap keys no-op on absent decks
- [x] Deck count persisted; session restore recreates snapshot decks; CLI track lands on deck 1
- [x] Dismiss confirmation on the global bar (destructive-confirm pattern)
- [x] keybindings.md: Alt+N select/summon/dismiss semantics

## Log

- Hand-back: `alt+N` dead on first test — the repo-root `config.toml` was ancient (pre-rename `select_deck1` action names, missing several current actions); dev-build-run.py deletes it by design but the binary had been launched directly. Removed; it regenerates from embedded defaults.
- Hand-back addition: the help overlay gains an Alt-layer footer row (`Alt: j/k/↑↓ cycle · 1/2/3 select/summon/dismiss · h/l panel · r restore`), replacing the two Alt fragments crowding the Space row.

- Browser-side `alt+N` is select/summon only — dismissal stays a deck-side gesture (dismissing the deck you're about to load onto, from inside the browser, is a foot-gun).
- Found in passing: the swap actions never moved in-flight pending loads (nor PFL routing). Swaps now move pending loads; PFL-on-swap left untouched as pre-existing behaviour, worth its own look.
- Existing sessions: `deck_count` defaults to 1 on first run after upgrade; a previous 3-deck session returns via `alt+r`, which summons the decks its snapshots need.
- Dismissal is a local macro so the per-slot attachments (PFL, pending loads, detached view, load-confirm) reflow in exactly one place.
- Hand-back bug: with the browser view showing but unfocused, the pre-focus-model Alt intercept still captured Alt chords and routed deck digits to select/summon only — dismissal unreachable. The intercept now requires browser focus; side effect, panel resize (`alt+h/l`) also needs focus now.
- Hand-back: create-but-not-destroy from the focused browser read as broken, overturning the earlier foot-gun guard — Alt+digit now means the same thing everywhere (one shared macro), and the `y` confirm intercepts ahead of browser key handling so the prompt works from either side.
- Hand-back: prompt verb settled as **destroy** ("Destroy deck N?"), and the vocabulary unified across the help overlay, keybindings.md, notifications, and the refusal message.

## Conclusion

Shipped as 0.34.4 (minor bump confirmed). The operator-facing verb settled as *destroy*, unified across prompt, messages, help, and docs. Map catch-up to follow: the root node's "Three decks", Deck Selection, Session State's persisted list, and the Keymap Alt-layer line.
