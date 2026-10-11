# Mode Shortcut Help

**Mode:** Formal

## Intent

Make shortcuts understandable in the active mode and focus context. Explain how hjkl changes meaning between deck controls and browser navigation, and expose grid-session actions such as a, which pins the grid anchor at the cursor instead of adjusting pitch. Make these context-dependent meanings discoverable in help and contextual hints.

## Log

- 2026-09-20: Parked at the user's request during keyboard-zoning review. The static keyboard map cannot fully explain mode- and focus-dependent bindings. Capture this for future work; no approach or implementation approved.

- 2026-09-21: Include browser command-mode backtick: it jumps to and highlights the selected deck's loaded track; an empty deck produces a notification. The full keyboard help omits it. The contextual hint currently says “loaded locations”, which obscures the selected-deck behaviour. Make this binding discoverable and clarify its label alongside the other mode-specific shortcuts.

- 2026-09-21: Backtick help omission and the ambiguous contextual label are now fixed within keyboard-zoning. The broader mode/focus-dependent help work, including grid a and browser hjkl, remains parked.
