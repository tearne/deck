# Guard Against Accidental Quit

**Mode:** Formal

## Intent

*(Parked — captured as an aside, then deepened while testing [[playlist-needs-confirmation]].)*

Two related problems around unintended quitting:

- **A single Esc can cascade two levels and close the browser.** With a playlist selected (Browse), one physical Esc deselects (Browse → `Preview::Empty`); the panel then auto-resyncs to `Preview::Playlist`; and because Esc isn't consumed in a Preview panel it falls through to the browser handler, closing the browser. The trigger is that one physical press delivers **two Esc events** (kitty `Repeat`, or a phantom second `Press`). The cascade itself is correct — one Esc per level is the desired UX — so the fix is to stop the UI ever seeing the phantom.

  A time-based debounce was tried and failed: the two events straddle a render, and frames aren't guaranteed under any fixed window, so the second event lands outside it and acts. (An inline sliding-window guard remains in `main.rs` as a partial mitigation from that attempt — to be removed by this change.)

  **Chosen direction:** an input-normalization component between the terminal and the UI whose sole job is to make **one physical Esc press yield one Esc event**. Core strategy is timing-free: collapse multiple Esc events within a single event-drain batch into one (a phantom from a quick press lands in the same batch). Where kitty delivers Release events, additionally latch on release for the held-key case. The UI then goes back to handling clean events, and the scattered top-level Esc guard is deleted.

  Suggested first step: a few lines of logging to confirm the phantom's event-kind and that it shares the drain batch, before committing.

- **Exit confirmation.** Quitting has no guard, so an accidental keystroke can drop the session. Require a confirmation step before the application exits. (The InputSource component is a natural host for this policy too.)
