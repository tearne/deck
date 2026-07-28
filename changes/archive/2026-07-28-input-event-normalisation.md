# Input Event Normalisation

**Mode:** Formal

## Intent

A recurring bug: dismissing an overlay with Esc arms the "quit while playing?" warning, because the Kitty keyboard protocol (via crossterm) sometimes decodes one physical keypress as two `Press` events. Esc is overloaded — it both dismisses overlays and is bound to Quit — so the phantom second Esc leaks past the just-closed overlay into the quit action.

Today this is worked around per-site: five separate `suppress_quit_until` assignments, one for each Esc-dismissable overlay, with every new overlay obliged to add its own. The same underlying quirk is patched independently again on the Space-chord path (`space_repeat_suppressed`). The workarounds are a ritual repeated at each call site, which is why the bug keeps reappearing whenever a new overlay is added.

Fix the cause once at the event-read boundary — collapse duplicate `Press` events for the same key — so no downstream handler sees the phantom event and the scattered per-site suppressions can be removed.

Possible extension (to weigh in the Approach): unify the overlays (help, file-ops submenu, quit-confirm, notification, browser) into a single modal stack so Esc is dispatched in one place rather than re-derived at each. This reduces "which overlay does this key hit" ambiguity but does not by itself fix the paired-Press, so it complements the boundary fix rather than replacing it.


## Approach

The captured event stream (0.11.16 diagnostic) rewrote the plan. Two facts, consistent across keyboards:

- Auto-repeat is delivered as a stream of `Press` events, not `Repeat` — there are no `Repeat` events at all. So a held key and the Esc phantom both look like "another Press soon after", and a key-agnostic boundary fix cannot tell them apart.
- The phantom is specific to Esc: one Esc tap yields two `Press` events ~74–110 ms apart and no `Release`, whereas ordinary keys get a clean `Press`/`Release` and deliberate Esc-to-quit lands seconds later.

### Swallow the phantom, but never block a deliberate re-press

An *acting* Esc opens a short window (~200 ms); a following Esc within it is the phantom and is dropped. Crucially the window is started only by an acting Esc and is never refreshed by the presses it swallows — so a deliberate re-press just after (e.g. impatiently pressing Esc to quit) always gets through once the window clears. The window only has to outlast the phantom's ~110 ms lag, not a whole hold.

Two earlier versions failed on hand-back and are abandoned: a 250 ms guard at the quit paths only (auto-repeat past 250 ms cascaded a browser-dismiss into a quit); and a 750 ms gesture window refreshed on every press (it coalesced a whole hold, but that same refresh meant repeated impatient Esc presses kept re-arming it, so the app could never be quit). Accepted limit: a very long Esc *hold* (past the ~500 ms auto-repeat delay) can still leak one press to the underlying layer — an unusual gesture, and the priority is that deliberate presses always work.

### Delete the scattered quit-suppression code

This replaces all five `suppress_quit_until` assignments and both checks: they existed only to absorb the phantom Esc at individual dismiss sites. The single Esc-timing guard covers every case, so a new overlay needs no quit-guard of its own — which was the real goal.

### Space-key handling and the modal-stack idea stay out

`space_repeat_suppressed` solves a different problem (is Space held as a modifier), and there is no clean boundary normalisation to fold it into anyway. The broader "one place for all Esc" refactor is unnecessary now that the per-site ritual is gone.


## Plan

- [x] Track the last Esc-press time; ignore an Esc within ~250 ms of it at the quit-arm and quit-confirm points.
- [x] Remove `suppress_quit_until` and its assignments and checks.
- [x] Remove the temporary `DECK_KEY_LOG` diagnostic.
- [x] Bump Cargo patch (0.11.16 → 0.11.17).


## Log

- The old comment at the `space_repeat_suppressed` declaration claims Release events "don't arrive in crossterm 0.29 + Kitty". Warp nudge (which ends on Release) works in Kitty, so releases do arrive for normal keys — the caveat reads as Space-specific. The held-key normalisation relies on releases for non-Space keys, which hold.
- Staged deliberately: 0.11.15 adds the normalisation with `suppress_quit_until` still present as a backstop, to verify the phantom is actually caught before the net is removed. The two coexist harmlessly — the normalisation drops the phantom Press, so the suppression simply never triggers.
- Verification (0.11.15, held-key attempt) failed: holding Esc in the browser then leaves Esc non-functional. The held-key set gets Esc stuck — a Release is missed or a phantom Press lands after the Release (consistent with the line-454 comment that releases are unreliable in crossterm 0.29 + Kitty), so Esc stays in the set and every later Press is dropped. That mechanism is unsound here.
- Re-approached (0.11.15, time-window): drop a fresh Press within ~100 ms of the same key's previous event. Also failed: the phantom Esc still armed quit (so it arrives >100 ms after the real press), yet 100 ms already swallowed legitimate fast taps. The phantom's delay overlaps real tapping speed, so no pure time threshold separates them. Both attempts were built on guesses about the event stream.
- Reverted; added a diagnostic instead (0.11.16): with `DECK_KEY_LOG` set, every key event (kind, code, mods, elapsed ms) is written to `key-events.log`. Need the real per-keypress event sequence before re-approaching — guessing has cost two attempts.
- Diagnostic captures (two keyboards) settled it: no `Repeat` events exist — auto-repeat comes as a Press stream ~18–36 ms apart with one trailing Release. Esc alone emits two Presses ~74–110 ms apart and no Release. So the phantom can't be separated from wanted repeats at the boundary, but it's Esc-specific.
- First fix attempt (0.11.17): 250 ms Esc debounce at the quit paths only. Failed on hand-back — holding Esc to dismiss the browser then quit the app: the browser closes on the first Esc, and auto-repeat (~500 ms in, past the 250 ms window) reaches the player as a fresh Esc → instant quit when nothing is playing.
- 0.11.18 (750 ms gesture window, refreshed on every Esc) also failed on hand-back: it stopped the browser-hold cascade, but because every press refreshed the window, a user impatiently pressing Esc to quit kept re-arming it — the app couldn't be quit at all.
- Fixed (0.11.19): window shortened to 200 ms and started only by an *acting* Esc, never refreshed by swallowed presses. Swallows just the phantom (~110 ms lag); any deliberate re-press after 200 ms acts normally. Trade-off accepted: a very long Esc hold can still leak one press past a dismissed overlay — deliberate-press reliability was chosen over blocking that rare gesture.


## Conclusion

Shipped at 0.11.19 as a small Esc debounce: after an Esc that acts, a second Esc within 200 ms is dropped (the duplicate the terminal emits for one keypress). This replaced all five scattered `suppress_quit_until` sites with one rule, so a new overlay no longer needs its own quit-guard — the recurring-bug goal. Not gated to any terminal; harmless where no duplicate occurs. Out of scope as planned: `space_repeat_suppressed` (a separate Space-hold concern) and the modal-stack refactor. Known limit: a very long Esc hold can leak one press to the layer beneath a just-dismissed overlay.
