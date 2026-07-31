# Deck-Independent Browser

**Mode:** Formal

## Intent

The browser is opened *for* a specific deck (the selected one at open time) and loading a track sends it to that deck. But the browser's other operations — rename, move, and the compliance work to come — are file operations with nothing to do with a deck. So the browser now does deck-agnostic file work while still being "owned" by a deck, which feels odd.

Make the browser independent of any deck. Loading is the only deck-specific action, so the **load action chooses the target deck at load time** — with the usual confirmation when the chosen deck is already playing — rather than the deck being fixed when the browser opens.

Captured as a follow-on to [[browser-file-operations]].


## Approach

### The browser is not bound to a deck; it carries a floating target

`open_browser` (`space+f`) always opens — the pre-open "deck is playing?" gate (`browser_blocked`) and the fixed "deck N" title are removed; opening never interrupts anything. Instead the browser holds a **target deck** — where a load will go — that floats rather than binding the browser: you browse, edit, and move any file regardless of it. The default picks the least-disruptive deck, lowest-numbered first: an empty deck if there is one, else a loaded-but-not-playing deck, and only the selected deck when all three are playing (where the load-time warning then applies). Shown in the browser.

### Enter loads into the target; the target is adjustable in any mode

`Enter` loads the highlighted (or search-result) track into the target deck — one key, identical in command and search mode. `[` and `]` cycle the target deck; being non-letter keys they work in search mode too, so the search-then-load flow needs no mode switch (search → find → `]` to retarget → `Enter`). In command mode, `1` / `2` / `3` set the target directly.


### The already-playing warning moves to load time

Loading into a deck that is currently playing raises a per-load confirmation (`y` load / `Esc`/`n` cancel), replacing the pre-open gate. You can always open and browse; the warning fires only when a load would actually interrupt a playing deck — the moment it matters.

### Edit and move are untouched

They already act on the highlighted file with no deck reference (deck-sync is by path), so the load path is the whole change.

### Esc backs out one level, without changing mode

Revises the modal browser's "Esc always exits" rule, which left no way to clear a search filter in place. `Esc` now pops one level: an active search filter is cleared **in the current mode** (the listing returns to the plain directory view, the mode unchanged); with no filter, `Esc` exits the browser; a Move cancels to Command. Not switching mode on clear is deliberate — it lets you exit *from* search mode, so the sticky primary-mode-on-reopen still restores search.


## Plan

- [x] Drop the target deck slot from `browser_state`; add a `target_deck` to `BrowserState`, set on open to the least-disruptive deck (empty → not-playing → selected).
- [x] `open_browser` always opens; remove the pre-open playing gate (`browser_blocked`).
- [x] `[`/`]` cycle the target deck in any mode; `1`/`2`/`3` set it in command mode; show the target in the browser (replacing the fixed "deck N" title).
- [x] `Enter` loads the highlighted or found track into the target deck (closing the browser, as today); loading into a playing deck asks a per-load confirmation.
- [x] `Esc` clears an active search filter in place (mode unchanged), else exits the browser.
- [x] Bump Cargo patch (0.11.25 → 0.11.26).


## Log

- `browser_state` is now `Option<BrowserState>` (target moved onto `BrowserState.target_deck`); the `browser_blocked` pre-open gate is gone — `open_browser` always opens and sets the target via `default_target_deck` (empty → not-playing → selected). The fixed "deck N" title became a mode-accent-coloured `→ deck N`.
- Target keys: `[`/`]` handled before the mode dispatch so they cycle the target in every mode (non-letters, no search conflict); `1`/`2`/`3` set it in command mode. Load is always `Enter`.
- Load into a playing deck defers behind `browser_load_confirm` with a `[y]/[n]` notification, intercepted at the top of the browser block; the browser stays open until confirmed. A non-playing target loads and closes immediately, as before.
- Esc: `has_filter()`-gated — clears the filter in place (mode unchanged, so exiting from search still restores search on reopen) when a filter is active, else exits. Applied in both command and search modes.
- Hand-back visibility pass (0.11.27): the target moved from a dim top-right title to a bold accent-coloured chip `▶ DECK N` on the top-left, with the mode content (search field / workspace hint / move banner) right-aligned. The selected-deck yellow highlight (accent bar + deck numbers) is suspended while the browser is open — player commands are intercepted then and the target chip is the only relevant deck marker, so the yellow no longer reads as the target.
- Load-into-playing-deck confirm, settled (0.11.29): the original `_ => {}` arm plus an expiring notification left input wedged. Final form is a top-bar warning "Deck N is playing — Enter to load, any other key cancels"; `Enter` loads, anything else cancels. No wedge, and the keys match the prompt exactly.


## Conclusion

Shipped at 0.11.29. The browser no longer opens *for* a deck: it holds a floating **target deck** (default: empty → not-playing → selected), shown as a bold `▶ DECK N` chip on the top-left, cycled with `[`/`]` in any mode or `1`/`2`/`3` in command mode. `Enter` loads into the target; loading into a playing deck raises a top-bar Enter-to-confirm warning. The pre-open playing gate is gone. `Esc` now backs out one level — clearing an active search filter in place (mode unchanged, so sticky-search survives) before exiting. And the selected-deck yellow highlight is suspended while the browser is open, so only the target chip signals a deck.

Hand-back took several visibility/behaviour iterations (target prominence, suspending the yellow highlight, and settling the load-confirm keys). Map catch-up pending on the Browser node: the floating target and its keys, and the Esc-clears-filter behaviour.
