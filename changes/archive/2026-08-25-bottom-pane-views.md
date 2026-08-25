# Bottom-Pane Views

**Mode:** Formal

## Intent

The browser is good enough that the operator sometimes wants to leave it open — visible, holding its place — while driving the decks with the keyboard as normal. Today the browser is a modal that swallows every key (only Alt chords pass through), so "browser open" and "deck keys work" are mutually exclusive.

Reframe the space at the bottom of the screen as holding one of four **views** — browser, album art, help, messages — as equal citizens: which view is showing is one choice, and whether the keyboard is talking to the browser or the decks is a separate one.

## Approach

### View and focus are two independent states

The bottom space always shows one of four views — album art (the ground state), browser, help, messages. Separately, the keyboard talks to either the decks or the showing view. Help and art take no input, so they never hold focus; browser and messages can.

### One chord per view, same meaning from either side

`space+f` browser, `space+h` help, `space+n` messages, `space+v` art. Each shows its view; browser and messages also take focus. Chosen to be free on the Space layer — `space+a` (pitch reset) and `space+m` (deck level) were rejected because a chord whose meaning depends on which pane holds focus is a trap when the operator forgets where focus is.

Bare `?` stays as an alias for the help chord; bare `N` is retired — messages are reached by `space+n` only. `space+v` shows the art view; further presses cycle its brightness. `/` is untouched (brightness at deck level, search in the browser) — overloading it with view-surfacing would have made its behaviour depend on context.

Two chords are vacated to make room: metronome toggle moves `space+v` → `space+\`, and `playlist_next`/`playlist_prev` lose their default keys (`space+n`/`space+p`) — the actions remain bindable, and the upcoming playlist change can revisit their home.

### Tab is the focus flip

Unbound at deck level; its three in-browser uses retire to their parallel keys (`/` for search, `l`/`h` in playlist editing, `Down`/`Up` in the tag editor) so Tab keeps one meaning. Tab also stops a running preview, per the existing any-other-key rule.

### Enter-to-load hands focus to the decks

The loaded deck is the selected one, so the operator drives it immediately. The browser stays showing; Tab returns to it for the next pick.

### Esc ladder unchanged

Focus back to the decks → close the view (back to art) → quit.

### The showing view persists

Remembered in session state alongside the browser directory and panel width. Focus starts with the decks on launch.

### The unfocused browser is a passive follower

A load highlights the loaded track in the browser, and the context panel shows its tag info — the browser is worth looking at even while the decks are being driven.

### Rename offer routes through the browser view

While the browser view is showing, the rename offer appears in the context panel rather than on the deck's notification bar — the deck stays unpolluted; the deck bar carries the offer only when the browser isn't showing. Accepting it switches to (or stays in) the browser at the file and focuses it — no special geometry for a browser-showing-but-unfocused state.

### Help and legends follow

The `?` overlay and the browser status legends gain the new vocabulary (Tab, the view chords); `#` preview joins the command-mode legend. The legends' `[ ] deck` and keybindings.md's `q`-closes-browser row match nothing in the code — deck selection in the browser is `Alt+j/k` — so both are fixed as stale.

### Visual focus treatment is iterated in Build

Focus must be unmistakable without looking ugly. The context panel's focus-by-light idiom is the starting point, not necessarily the answer.


## Plan

- [x] Introduce the four-view bottom space: art as ground state; browser, help, messages as the other views
- [x] Introduce deck/view keyboard focus with Tab as the flip; help and art never take focus
- [x] Bind the view chords: `space+f`, `space+h` (bare `?` alias), `space+n` (bare `N` retired), `space+v` with repeat presses cycling art brightness
- [x] Retire Tab's in-browser uses to their parallel keys (search toggle, playlist-edit pane switch, tag-editor fields)
- [x] Enter-to-load hands focus to the decks
- [x] Esc ladder: view focus → close to art → quit
- [x] Stop a running preview when Tab leaves the browser
- [x] Persist the showing view in Session State; focus starts with the decks on launch
- [x] Passive follow: a load highlights the track in the unfocused browser, the context panel showing its tags
- [x] Rename offer renders in the context panel while the browser view shows, on the deck notification bar otherwise
- [x] Focus treatment: whichever side holds the keyboard is bright, the other dimmed — refined with the user at hand-back
- [x] Update the `?` overlay and browser legends: view vocabulary, `#` preview added, stale `[ ] deck` and `q` entries fixed
- [x] Catch up keybindings.md with the new and retired bindings
- [x] Help and messages views draw over the art ground (hand-back tweak)
- [x] Help layout gains the middle key columns — 6/Y/H/N (hand-back tweak)

## Log

- Space-in-search conflict: in browser search mode, Space is a typeable filter character, so Space chords can't fire there without eating typed spaces. View chords will work from deck focus and browser command mode; from search mode, Tab out first. To confirm at hand-back.
- Blocker: the approved chords clashed with existing bindings — `space+n` was `playlist_next`, `space+v` was `metronome_toggle` (the earlier free-key check was too narrow). Resolved: metronome moves to `space+\`, playlist next/prev defaults removed, original view chords stand.
- keybindings.md's layout diagram shows `BDtct` on B's Space layer, but nothing is bound there in config — stale cell, to fix with the other doc catch-ups.
- Browser search mode: Esc with no filter now backs out to command mode (previously it exited the browser) — the level Tab's mode-toggle used to provide.
- Esc-quit always steps through the art ground state, so a quit via Esc persists "art" as the view; only Ctrl-C (or the playing-quit confirm) can persist browser/help/messages. To discuss at hand-back.
- keybindings.md advertised `q` closes the browser — no code handles `q`; row removed rather than implemented.
- layout.md's empty-deck hint read `Space+D`; the bound key is `Space+F` — corrected.
- The rename-offer banner in the context panel names the file (`⚠ rename Foo.mp3? [y]`), since the highlight may sit elsewhere; deck-corner ⚠ and countdown are suppressed while the browser view shows.
- A session-restored browser view suppresses the startup "opens the file browser" hint.
- The help overlay advertised `Sp+1/2/3 SelD1-3` — no direct deck-select action exists in the code (only next/prev cycling); stale cells removed from the overlay and keybindings.md. Relevant to the parked deck-selection-keys proposal.
- The middle key columns surface `^`/`Y` as Shift+6/Shift+y FPS cells — previously the FPS keys appeared only in the config table, not the overlay.
- Help layout: the left/right block divider (`┆`) removed — with the middle columns filled in, the grid reads as one keyboard; the mixer trio's compact cells still group it. keybindings.md prose updated to match.
- Hand-back: `?` did nothing while the browser had focus (bare keys stop at the browser's own handler). Fixed: in command mode a bare key bound to `help` switches views like `space+h`; in search mode it stays a typed character.
- Hand-back: the metadata pane was dimmed twice when the browser region was unfocused (its passive-follow fade plus the region fade). Now the passive-follow/driven fades apply only while the browser holds the keyboard; unfocused, the single region fade stands alone. Foreground fade eased 0.55 → 0.65 for legibility.
- Focus treatment first cut: unfocused browser/messages regions get the existing hue-preserving dim over the whole area; decks are never dimmed. Refinement expected at hand-back.

## Conclusion

Shipped as 0.32.4 (minor bump confirmed; patch iterations covered the hand-back tweaks). Beyond the Log: keybindings.md and layout.md were caught up in-build; the help overlay grew the middle key columns and lost its block divider — the follow-on zoning idea is parked as [keyboard-zoning](../open/keyboard-zoning.md). Esc-quit persists the art view by design of the Esc ladder; accepted as built. Map catch-up (Browser, Keymap, Album Art, Messages, Session State) follows as per-node negotiation.
