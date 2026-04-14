# Kitty Keyboard Protocol

## Note for Planner

During the fps-control build, a bug was fixed where Kitty was injecting a duplicate Esc Press event immediately after the first, silently cancelling the quit-confirmation bar. The fix was a workaround (`suppress_quit_until`), consistent with how the same problem is already handled on overlay-close paths.

The root cause is well-documented and intentional: deck uses `REPORT_EVENT_TYPES` which provides Press/Release/Repeat distinction but does not fully disambiguate the Esc byte (`0x1b`), which is ambiguous in legacy terminal encoding because it also starts escape sequences. Kitty's full keyboard protocol (`CSI u`, aka `DISAMBIGUATE_ESC_CODES`) solves this cleanly but requires a further opt-in.

The workaround pattern (`suppress_quit_until`) is now applied consistently across all Esc-triggered paths. This is likely sufficient. However, if the duplicate-event problem surfaces again in other paths, opting into `DISAMBIGUATE_ESC_CODES` alongside `REPORT_EVENT_TYPES` may be worth considering as a more permanent fix.

**References**
- Kitty keyboard protocol: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
- Ambiguous escape codes discussion: https://github.com/kovidgoyal/kitty/discussions/4778
