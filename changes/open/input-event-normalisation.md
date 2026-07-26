# Input Event Normalisation

**Mode:** Formal

## Intent

A recurring bug: dismissing an overlay with Esc arms the "quit while playing?" warning, because the Kitty keyboard protocol (via crossterm) sometimes decodes one physical keypress as two `Press` events. Esc is overloaded — it both dismisses overlays and is bound to Quit — so the phantom second Esc leaks past the just-closed overlay into the quit action.

Today this is worked around per-site: five separate `suppress_quit_until` assignments, one for each Esc-dismissable overlay, with every new overlay obliged to add its own. The same underlying quirk is patched independently again on the Space-chord path (`space_repeat_suppressed`). The workarounds are a ritual repeated at each call site, which is why the bug keeps reappearing whenever a new overlay is added.

Fix the cause once at the event-read boundary — collapse duplicate `Press` events for the same key — so no downstream handler sees the phantom event and the scattered per-site suppressions can be removed.

Possible extension (to weigh in the Approach): unify the overlays (help, file-ops submenu, quit-confirm, notification, browser) into a single modal stack so Esc is dispatched in one place rather than re-derived at each. This reduces "which overlay does this key hit" ambiguity but does not by itself fix the paired-Press, so it complements the boundary fix rather than replacing it.
